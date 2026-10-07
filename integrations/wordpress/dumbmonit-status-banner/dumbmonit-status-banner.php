<?php
/**
 * Plugin Name: DumbMonit Status Banner
 * Description: Shows a banner on your site when services on your DumbMonit status page are down.
 * Version: 1.0.0
 * Requires at least: 5.8
 * Requires PHP: 7.4
 * License: Apache-2.0
 * License URI: https://www.apache.org/licenses/LICENSE-2.0
 * Text Domain: dumbmonit-status-banner
 */

if (!defined('ABSPATH')) {
    exit;
}

const DMT_OPT = 'dumbmonit_banner';

function dmt_settings(): array
{
    $o = get_option(DMT_OPT, []);
    return wp_parse_args(is_array($o) ? $o : [], ['url' => '', 'position' => 'top', 'show' => 'issues']);
}

/** Only http(s) URLs of the form https://host/api/public/status/slug. */
function dmt_sanitize($in): array
{
    $in = is_array($in) ? $in : [];
    $url = esc_url_raw(trim((string) ($in['url'] ?? '')), ['http', 'https']);
    return [
        'url' => preg_match('#^https?://[^/]+/api/public/status/[a-z0-9-]+/?$#', $url) ? untrailingslashit($url) : '',
        'position' => ($in['position'] ?? '') === 'bottom' ? 'bottom' : 'top',
        'show' => ($in['show'] ?? '') === 'always' ? 'always' : 'issues',
    ];
}

add_action('admin_init', function () {
    register_setting('dmt_group', DMT_OPT, ['sanitize_callback' => 'dmt_sanitize']);
});

add_action('admin_menu', function () {
    add_options_page('DumbMonit Status Banner', 'DumbMonit Banner', 'manage_options', 'dumbmonit-banner', 'dmt_page');
});

function dmt_page(): void
{
    if (!current_user_can('manage_options')) {
        return;
    }
    $s = dmt_settings();
    ?>
    <div class="wrap">
      <h1>DumbMonit Status Banner</h1>
      <form method="post" action="options.php">
        <?php settings_fields('dmt_group'); ?>
        <table class="form-table" role="presentation">
          <tr><th><label for="dmt-url">Status page API URL</label></th>
            <td><input id="dmt-url" class="regular-text" type="url" name="<?php echo esc_attr(DMT_OPT); ?>[url]"
                value="<?php echo esc_attr($s['url']); ?>" placeholder="https://monit.example.com/api/public/status/my-page">
              <p class="description">The <code>/api/public/status/&lt;address&gt;</code> address of a published page.</p></td></tr>
          <tr><th><label for="dmt-pos">Position</label></th>
            <td><select id="dmt-pos" name="<?php echo esc_attr(DMT_OPT); ?>[position]">
              <option value="top" <?php selected($s['position'], 'top'); ?>>Top</option>
              <option value="bottom" <?php selected($s['position'], 'bottom'); ?>>Bottom</option></select></td></tr>
          <tr><th><label for="dmt-show">Show</label></th>
            <td><select id="dmt-show" name="<?php echo esc_attr(DMT_OPT); ?>[show]">
              <option value="issues" <?php selected($s['show'], 'issues'); ?>>Only when something is wrong</option>
              <option value="always" <?php selected($s['show'], 'always'); ?>>Always</option></select></td></tr>
        </table>
        <?php submit_button(); ?>
      </form>
    </div>
    <?php
}

/** Cached server-side fetch (one minute). Null when unreachable. */
function dmt_status(string $url): ?array
{
    $key = 'dmt_' . md5($url);
    $cached = get_transient($key);
    if (is_array($cached)) {
        return $cached;
    }
    $r = wp_safe_remote_get($url, ['timeout' => 3, 'redirection' => 0, 'limit_response_size' => 1048576,
        'headers' => ['Accept' => 'application/json']]);
    if (is_wp_error($r) || wp_remote_retrieve_response_code($r) !== 200) {
        return null;
    }
    $data = json_decode(wp_remote_retrieve_body($r), true);
    if (!is_array($data)) {
        return null;
    }
    set_transient($key, $data, MINUTE_IN_SECONDS);
    return $data;
}

function dmt_render(): void
{
    static $done = false;
    $s = dmt_settings();
    if ($done || $s['url'] === '' || is_admin()) {
        return;
    }
    $data = dmt_status($s['url']);
    if (!$data) {
        return;
    }
    $n = 0;
    foreach (($data['groups'] ?? []) as $g) {
        foreach (($g['items'] ?? []) as $i) {
            if (in_array($i['state'] ?? '', ['down', 'degraded'], true)) {
                $n++;
            }
        }
    }
    $overall = (string) ($data['overall'] ?? '');
    if ($overall === 'maintenance') {
        $c = ['#dbeafe', '#1e3a8a'];
        $text = 'Maintenance in progress';
    } elseif ($n > 0) {
        $c = $overall === 'major' ? ['#fee2e2', '#7f1d1d'] : ['#fef3c7', '#78350f'];
        /* translators: %d number of services */
        $text = $n === 1 ? '1 service is down' : sprintf('%d services are down', $n);
    } elseif ($overall !== 'operational') {
        $c = ['#fef3c7', '#78350f'];
        $text = 'Incident in progress';
    } elseif ($s['show'] === 'always') {
        $c = ['#e6f4ea', '#14532d'];
        $text = 'All systems operational';
    } else {
        return;
    }
    $done = true;
    $p = parse_url($s['url']);
    $link = $p['scheme'] . '://' . $p['host'] . (isset($p['port']) ? ':' . (int) $p['port'] : '')
        . '/s/' . rawurlencode((string) ($data['page']['slug'] ?? ''));
    $pos = $s['position'] === 'bottom' ? 'bottom' : 'top';
    printf(
        '<div role="status" style="position:fixed;left:0;right:0;%s:0;z-index:99999;padding:10px 16px;text-align:center;font:14px/1.4 system-ui,sans-serif;background:%s;color:%s">%s &middot; <a style="color:inherit" href="%s" rel="noopener">%s</a></div>',
        esc_attr($pos),
        esc_attr($c[0]),
        esc_attr($c[1]),
        esc_html($text),
        esc_url($link),
        esc_html__('Details', 'dumbmonit-status-banner')
    );
}

add_action('wp_body_open', 'dmt_render');
add_action('wp_footer', 'dmt_render'); // themes without wp_body_open; runs once
