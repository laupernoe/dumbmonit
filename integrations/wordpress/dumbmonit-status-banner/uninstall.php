<?php
if (!defined('WP_UNINSTALL_PLUGIN')) {
    exit;
}
$opt = get_option('dumbmonit_banner');
if (is_array($opt) && !empty($opt['url'])) {
    delete_transient('dmt_' . md5($opt['url']));
}
delete_option('dumbmonit_banner');
