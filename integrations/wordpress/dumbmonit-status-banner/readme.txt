=== DumbMonit Status Banner ===
Contributors: dumbmonit
Tags: status, uptime, monitoring, banner
Requires at least: 5.8
Tested up to: 6.8
Requires PHP: 7.4
Stable tag: 1.0.0
License: Apache-2.0
License URI: https://www.apache.org/licenses/LICENSE-2.0

Shows a banner on your site when services on your DumbMonit status page are down.

== Description ==

Reads the public JSON of a published DumbMonit status page on the server (cached for one minute) and prints a banner such as "2 services are down", or "All systems operational" if you choose to always show it. Nothing is loaded from the visitor's browser.

== Installation ==

1. Copy the `dumbmonit-status-banner` folder to `wp-content/plugins/` and activate it.
2. Go to Settings > DumbMonit Banner.
3. Enter the status page API URL, for example `https://monit.example.com/api/public/status/my-page`, choose position and show mode.

== Changelog ==

= 1.0.0 =
* First release.
