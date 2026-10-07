<?php
/**
 * DumbMonit status banner, server side, no framework.
 *
 *   require __DIR__ . '/banner.php';
 *   dumbmonit_banner('https://monit.example.com/api/public/status/my-page');
 *
 * Options: 'position' => 'top'|'bottom', 'show' => 'issues'|'always',
 *          'ttl' => seconds of file cache (default 60),
 *          'cache_dir' => writable directory (default: system temp dir).
 * Prints nothing when everything is fine (show=issues) or when the server
 * cannot be reached. Every value from the server is escaped.
 */

function dumbmonit_fetch(string $url, int $ttl, string $cacheDir): ?array
{
    $parts = parse_url($url);
    if (!$parts || !in_array($parts['scheme'] ?? '', ['http', 'https'], true) || empty($parts['host'])) {
        return null; // only http(s) URLs
    }
    $file = rtrim($cacheDir, '/\\') . '/dumbmonit-banner-' . sha1($url) . '.json';
    if (is_file($file) && time() - (int) filemtime($file) < $ttl) {
        $raw = @file_get_contents($file);
    } else {
        $ctx = stream_context_create([
            'http' => ['timeout' => 3, 'follow_location' => 0, 'ignore_errors' => false,
                'header' => "Accept: application/json\r\nUser-Agent: dumbmonit-banner-php\r\n"],
            'ssl' => ['verify_peer' => true, 'verify_peer_name' => true],
        ]);
        $raw = @file_get_contents($url, false, $ctx, 0, 1048576);
        if ($raw === false) {
            // Serve a stale copy rather than nothing.
            $raw = is_file($file) ? @file_get_contents($file) : false;
        } else {
            @file_put_contents($file, $raw, LOCK_EX);
        }
    }
    $data = is_string($raw) ? json_decode($raw, true) : null;
    return is_array($data) ? $data : null;
}

function dumbmonit_banner(string $url, array $opts = []): void
{
    $data = dumbmonit_fetch($url, (int) ($opts['ttl'] ?? 60), $opts['cache_dir'] ?? sys_get_temp_dir());
    if ($data === null) {
        return;
    }
    $down = 0;
    foreach (($data['groups'] ?? []) as $group) {
        foreach (($group['items'] ?? []) as $item) {
            if (in_array($item['state'] ?? '', ['down', 'degraded'], true)) {
                $down++;
            }
        }
    }
    $overall = (string) ($data['overall'] ?? '');
    if ($overall === 'maintenance') {
        $tone = 'info';
        $text = 'Maintenance in progress';
    } elseif ($down > 0) {
        $tone = $overall === 'major' ? 'bad' : 'warn';
        $text = $down === 1 ? '1 service is down' : $down . ' services are down';
    } elseif ($overall !== 'operational') {
        $tone = 'warn';
        $text = 'Incident in progress';
    } else {
        if (($opts['show'] ?? 'issues') !== 'always') {
            return;
        }
        $tone = 'ok';
        $text = 'All systems operational';
    }
    $pos = ($opts['position'] ?? 'top') === 'bottom' ? 'bottom' : 'top';
    $e = static fn(string $s): string => htmlspecialchars($s, ENT_QUOTES | ENT_SUBSTITUTE, 'UTF-8');
    $slug = (string) ($data['page']['slug'] ?? '');
    $parts = parse_url($url);
    $port = isset($parts['port']) ? ':' . (int) $parts['port'] : '';
    $link = $parts['scheme'] . '://' . $parts['host'] . $port . '/s/' . rawurlencode($slug);

    $colors = [
        'ok' => ['#e6f4ea', '#14532d'], 'warn' => ['#fef3c7', '#78350f'],
        'bad' => ['#fee2e2', '#7f1d1d'], 'info' => ['#dbeafe', '#1e3a8a'],
    ][$tone];
    echo '<div class="dmt-banner" role="status" style="position:fixed;left:0;right:0;', $pos,
        ':0;z-index:2147483000;padding:10px 16px;text-align:center;font:14px/1.4 system-ui,sans-serif;',
        'background:', $colors[0], ';color:', $colors[1], '">',
        $e($text), ' &middot; <a style="color:inherit" href="', $e($link),
        '" rel="noopener">Details</a></div>', "\n";
}
