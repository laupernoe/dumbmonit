<?php
/**
 * Plugin Name: DumbMonit Status Banner
 * Description: Shows a banner on your site when services on your DumbMonit status page are down.
 * Version: 1.1.0
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
const DMT_ERR = 'dmt_last_error';

add_action('init', function () {
    load_plugin_textdomain('dumbmonit-status-banner', false, dirname(plugin_basename(__FILE__)) . '/languages');
});

/** Strict form of the accepted address: http(s)://host[:port]/api/public/status/<slug>. */
function dmt_valid_url(string $url): bool
{
    return (bool) preg_match('#^https?://[^/\s]+/api/public/status/[a-z0-9-]+/?$#', $url);
}

function dmt_settings(): array
{
    $o = get_option(DMT_OPT, []);
    return wp_parse_args(is_array($o) ? $o : [], ['url' => '', 'position' => 'top', 'show' => 'issues']);
}

/** Only http(s) URLs of the form https://host/api/public/status/slug. */
function dmt_sanitize($in): array
{
    $in = is_array($in) ? $in : [];
    $raw = trim((string) ($in['url'] ?? ''));
    $url = esc_url_raw($raw, ['http', 'https']);
    $ok = dmt_valid_url($url);
    if ($raw !== '' && !$ok) {
        add_settings_error(DMT_OPT, 'dmt_bad_url', __('Invalid status page URL. Expected https://your-server/api/public/status/<address>. The URL was not saved.', 'dumbmonit-status-banner'));
    }
    if ($ok) {
        delete_transient('dmt_' . md5(untrailingslashit($url)));
        delete_transient(DMT_ERR);
        if (dmt_status(untrailingslashit($url)) === null) {
            add_settings_error(DMT_OPT, 'dmt_unreachable', sprintf(
                /* translators: %s: error detail */
                __('Saved, but the status page could not be fetched from this server: %s', 'dumbmonit-status-banner'),
                esc_html((string) get_transient(DMT_ERR))
            ), 'warning');
        }
    }
    return [
        // An invalid entry keeps the previous value instead of silently emptying the setting.
        'url' => $ok ? untrailingslashit($url) : ($raw === '' ? '' : dmt_settings()['url']),
        'position' => ($in['position'] ?? '') === 'bottom' ? 'bottom' : 'top',
        'show' => ($in['show'] ?? '') === 'always' ? 'always' : 'issues',
    ];
}

add_action('admin_init', function () {
    register_setting('dmt_group', DMT_OPT, ['sanitize_callback' => 'dmt_sanitize']);
});

add_action('admin_notices', function () {
    if (!current_user_can('manage_options') || dmt_settings()['url'] === '') {
        return;
    }
    $err = get_transient(DMT_ERR);
    if (is_string($err) && $err !== '') {
        printf(
            '<div class="notice notice-warning"><p>%s</p></div>',
            esc_html(sprintf(
                /* translators: %s: error detail */
                __('DumbMonit Status Banner: cannot fetch the status page (%s). No banner is shown.', 'dumbmonit-status-banner'),
                $err
            ))
        );
    }
});

add_action('admin_menu', function () {
    add_options_page(__('DumbMonit Status Banner', 'dumbmonit-status-banner'), __('DumbMonit Banner', 'dumbmonit-status-banner'), 'manage_options', 'dumbmonit-banner', 'dmt_page');
});

function dmt_page(): void
{
    if (!current_user_can('manage_options')) {
        return;
    }
    $s = dmt_settings();
    ?>
    <div class="wrap">
      <h1><?php esc_html_e('DumbMonit Status Banner', 'dumbmonit-status-banner'); ?></h1>
      <form method="post" action="options.php">
        <?php settings_fields('dmt_group'); ?>
        <table class="form-table" role="presentation">
          <tr><th><label for="dmt-url"><?php esc_html_e('Status page API URL', 'dumbmonit-status-banner'); ?></label></th>
            <td><input id="dmt-url" class="regular-text" type="url" name="<?php echo esc_attr(DMT_OPT); ?>[url]"
                value="<?php echo esc_attr($s['url']); ?>" placeholder="https://monit.example.com/api/public/status/my-page">
              <p class="description"><?php echo wp_kses(__('The <code>/api/public/status/&lt;address&gt;</code> address of a published page. A private (LAN) address is allowed.', 'dumbmonit-status-banner'), ['code' => []]); ?></p></td></tr>
          <tr><th><label for="dmt-pos"><?php esc_html_e('Position', 'dumbmonit-status-banner'); ?></label></th>
            <td><select id="dmt-pos" name="<?php echo esc_attr(DMT_OPT); ?>[position]">
              <option value="top" <?php selected($s['position'], 'top'); ?>><?php esc_html_e('Top', 'dumbmonit-status-banner'); ?></option>
              <option value="bottom" <?php selected($s['position'], 'bottom'); ?>><?php esc_html_e('Bottom', 'dumbmonit-status-banner'); ?></option></select></td></tr>
          <tr><th><label for="dmt-show"><?php esc_html_e('Show', 'dumbmonit-status-banner'); ?></label></th>
            <td><select id="dmt-show" name="<?php echo esc_attr(DMT_OPT); ?>[show]">
              <option value="issues" <?php selected($s['show'], 'issues'); ?>><?php esc_html_e('Only when something is wrong', 'dumbmonit-status-banner'); ?></option>
              <option value="always" <?php selected($s['show'], 'always'); ?>><?php esc_html_e('Always', 'dumbmonit-status-banner'); ?></option></select></td></tr>
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
    // The URL is the one the admin configured, re-checked against the strict pattern. wp_remote_get
    // (not the "safe" variant, which refuses LAN addresses) so a DumbMonit on a private IP works.
    // Redirections are disabled so the request cannot be bounced to another host.
    if (!dmt_valid_url($url)) {
        return null;
    }
    $r = wp_remote_get($url, ['timeout' => 3, 'redirection' => 0, 'limit_response_size' => 1048576,
        'headers' => ['Accept' => 'application/json']]);
    if (is_wp_error($r)) {
        set_transient(DMT_ERR, $r->get_error_message(), MINUTE_IN_SECONDS);
        return null;
    }
    if (wp_remote_retrieve_response_code($r) !== 200) {
        set_transient(DMT_ERR, 'HTTP ' . wp_remote_retrieve_response_code($r), MINUTE_IN_SECONDS);
        return null;
    }
    $data = json_decode(wp_remote_retrieve_body($r), true);
    if (!is_array($data)) {
        set_transient(DMT_ERR, 'invalid JSON', MINUTE_IN_SECONDS);
        return null;
    }
    delete_transient(DMT_ERR);
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
        $text = __('Maintenance in progress', 'dumbmonit-status-banner');
    } elseif ($n > 0) {
        $c = $overall === 'major' ? ['#fee2e2', '#7f1d1d'] : ['#fef3c7', '#78350f'];
        $text = sprintf(
            /* translators: %d: number of services down or degraded */
            _n('%d service is down', '%d services are down', $n, 'dumbmonit-status-banner'),
            $n
        );
    } elseif ($overall !== 'operational') {
        $c = ['#fef3c7', '#78350f'];
        $text = __('Incident in progress', 'dumbmonit-status-banner');
    } elseif ($s['show'] === 'always') {
        $c = ['#e6f4ea', '#14532d'];
        $text = __('All systems operational', 'dumbmonit-status-banner');
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
