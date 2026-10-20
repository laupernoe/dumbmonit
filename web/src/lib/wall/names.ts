/** Display names of the wall's widgets and zones (Settings → Wall widgets, layout). */
import { m } from '#lib/paraglide/messages.js';
import type { WallWidgetId, WallZone } from './layout.js';

export const WIDGET_NAMES: Record<WallWidgetId, () => string> = {
	weather: m.wall_widgets_weather,
	news: m.wall_widgets_news,
	cve: m.wall_widgets_cve,
	crypto: m.wall_widgets_crypto,
	stocks: m.wall_widgets_stocks,
	fx: m.wall_widgets_fx,
	claude: m.wall_widgets_claude,
	openai: m.wall_widgets_openai,
	text: m.wall_widgets_text
};

export const ZONE_NAMES: Record<WallZone, () => string> = {
	top: m.settings_widgets_zone_top,
	bottom: m.settings_widgets_zone_bottom,
	right: m.settings_widgets_zone_right,
	hidden: m.settings_widgets_zone_hidden
};
