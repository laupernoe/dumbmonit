//! Sources d'actualités proposées par pays pour le mode mur.
//!
//! Des données, pas de la logique : chaque flux a été relu à la main le
//! 14/10/2026 (HTTP 200, document RSS/Atom lisible, articles présents) et chaque
//! direct YouTube vérifié « en cours » par son identifiant de chaîne. Un flux qui
//! cesserait de répondre n'est pas un bogue du serveur : l'administrateur voit
//! l'erreur dans ses réglages et peut en saisir un autre (« flux personnalisé »).
//!
//! Les directs s'intègrent par chaîne (`embed/live_stream?channel=UC…`), donc
//! suivent le direct courant de la chaîne sans qu'on ait à connaître la vidéo.
//! Pas de direct proposé quand aucune chaîne officielle n'en diffuse un de façon
//! stable (Suisse, Belgique, Allemagne, Espagne, Italie, Portugal, Russie).

use serde::Serialize;

#[derive(Serialize)]
pub struct FeedPreset {
    pub label: &'static str,
    pub url: &'static str,
}

#[derive(Serialize)]
pub struct VideoPreset {
    pub label: &'static str,
    /// Lien de page YouTube de la chaîne, que `embed_url` sait lire.
    pub url: &'static str,
}

#[derive(Serialize)]
pub struct Country {
    /// Code ISO 3166-1 alpha-2, ou `INT` pour les sources internationales.
    pub code: &'static str,
    pub feeds: &'static [FeedPreset],
    pub videos: &'static [VideoPreset],
}

const fn feed(label: &'static str, url: &'static str) -> FeedPreset {
    FeedPreset { label, url }
}

const fn live(label: &'static str, url: &'static str) -> VideoPreset {
    VideoPreset { label, url }
}

pub static COUNTRIES: &[Country] = &[
    Country {
        code: "FR",
        feeds: &[
            feed("BFM TV", "https://www.bfmtv.com/rss/news-24-7/"),
            feed("franceinfo", "https://www.francetvinfo.fr/titres.rss"),
            feed("Le Monde", "https://www.lemonde.fr/rss/une.xml"),
            feed("France 24", "https://www.france24.com/fr/rss"),
        ],
        videos: &[
            live("BFM TV", "https://www.youtube.com/channel/UCXwDLMDV86ldKoFVc_g8P0g/live"),
            live("franceinfo", "https://www.youtube.com/channel/UCO6K_kkdP-lnSCiO3tPx7WA/live"),
            live("France 24", "https://www.youtube.com/channel/UCCCPCZNChQdGa9EkATeye4g/live"),
            live("Euronews", "https://www.youtube.com/channel/UCW2QcKZiU8aUGg4yxCIditg/live"),
        ],
    },
    Country {
        code: "CH",
        feeds: &[
            feed("RTS Info", "https://www.rts.ch/info/?format=rss/news"),
            feed("SRF News", "https://www.srf.ch/news/bnf/rss/1646"),
            feed("Le Temps", "https://www.letemps.ch/articles.rss"),
            feed("NZZ", "https://www.nzz.ch/recent.rss"),
        ],
        videos: &[],
    },
    Country {
        code: "BE",
        feeds: &[
            feed("RTBF", "https://rss.rtbf.be/article/rss/highlight_rtbf_info.xml"),
            feed("VRT NWS", "https://www.vrt.be/vrtnws/nl.rss.articles.xml"),
            feed("La DH", "https://www.dhnet.be/arc/outboundfeeds/rss/?outputType=xml"),
        ],
        videos: &[],
    },
    Country {
        code: "DE",
        feeds: &[
            feed("Tagesschau", "https://www.tagesschau.de/index~rss2.xml"),
            feed("ZDFheute", "https://www.zdf.de/rss/zdf/nachrichten"),
            feed("Spiegel", "https://www.spiegel.de/schlagzeilen/index.rss"),
            feed("Deutsche Welle", "https://rss.dw.com/rdf/rss-de-all"),
        ],
        videos: &[live("DW News", "https://www.youtube.com/channel/UCknLrEdhRCp1aegoMqRaCZg/live")],
    },
    Country {
        code: "GB",
        feeds: &[
            feed("BBC News", "https://feeds.bbci.co.uk/news/rss.xml"),
            feed("The Guardian", "https://www.theguardian.com/uk/rss"),
        ],
        videos: &[live(
            "Sky News",
            "https://www.youtube.com/channel/UCoMdktPbSTixAyNGwb-UYkQ/live",
        )],
    },
    Country {
        code: "US",
        feeds: &[
            feed("NPR", "https://feeds.npr.org/1001/rss.xml"),
            feed("PBS NewsHour", "https://www.pbs.org/newshour/feeds/rss/headlines"),
            feed("CBS News", "https://www.cbsnews.com/latest/rss/main"),
            feed("The New York Times", "https://rss.nytimes.com/services/xml/rss/nyt/HomePage.xml"),
        ],
        videos: &[live(
            "ABC News Live",
            "https://www.youtube.com/channel/UCBi2mrWuNuyYy4gbM6fU18Q/live",
        )],
    },
    Country {
        code: "CA",
        feeds: &[
            feed("Radio-Canada", "https://ici.radio-canada.ca/rss/4159"),
            feed("CBC News", "https://www.cbc.ca/webfeed/rss/rss-topstories"),
        ],
        videos: &[live(
            "CBC News",
            "https://www.youtube.com/channel/UCuFFtHWoLl5fauMMD5Ww2jA/live",
        )],
    },
    Country {
        code: "ES",
        feeds: &[
            feed("RTVE", "https://www.rtve.es/rss/temas_noticias.xml"),
            feed("20minutos", "https://www.20minutos.es/rss/"),
            feed("El País", "https://feeds.elpais.com/mrss-s/pages/ep/site/elpais.com/portada"),
            feed("El Mundo", "https://e00-elmundo.uecdn.es/elmundo/rss/portada.xml"),
        ],
        videos: &[],
    },
    Country {
        code: "IT",
        feeds: &[
            feed("ANSA", "https://www.ansa.it/sito/ansait_rss.xml"),
            feed("RaiNews", "https://www.rainews.it/rss/tutti"),
        ],
        videos: &[],
    },
    Country {
        code: "PT",
        feeds: &[
            feed("RTP Notícias", "https://www.rtp.pt/noticias/rss"),
            feed("Público", "https://feeds.feedburner.com/PublicoRSS"),
        ],
        videos: &[],
    },
    Country {
        code: "BR",
        feeds: &[
            feed("G1", "https://g1.globo.com/rss/g1/"),
            feed("Folha de S.Paulo", "https://feeds.folha.uol.com.br/emcimadahora/rss091.xml"),
        ],
        videos: &[live(
            "CNN Brasil",
            "https://www.youtube.com/channel/UCvdwhh_fDyWccR42-rReZLw/live",
        )],
    },
    Country {
        code: "RU",
        feeds: &[
            feed("Interfax", "https://www.interfax.ru/rss.asp"),
            feed("Meduza", "https://meduza.io/rss/all"),
            feed("Kommersant", "https://www.kommersant.ru/RSS/main.xml"),
        ],
        videos: &[],
    },
    Country {
        code: "CN",
        feeds: &[
            feed("CGTN", "https://www.cgtn.com/subscribe/rss/section/world.xml"),
            feed("South China Morning Post", "https://www.scmp.com/rss/91/feed"),
            feed("Xinhua", "http://www.xinhuanet.com/english/rss/worldrss.xml"),
        ],
        videos: &[live("CGTN", "https://www.youtube.com/channel/UCgrNz-aDmcr2uuto8_DL2jg/live")],
    },
    Country {
        code: "INT",
        feeds: &[
            feed("Deutsche Welle", "https://rss.dw.com/rdf/rss-en-all"),
            feed("France 24", "https://www.france24.com/en/rss"),
            feed("BBC News (world)", "https://feeds.bbci.co.uk/news/world/rss.xml"),
        ],
        videos: &[
            live("DW News", "https://www.youtube.com/channel/UCknLrEdhRCp1aegoMqRaCZg/live"),
            live(
                "France 24 English",
                "https://www.youtube.com/channel/UCQfwfsi5VrQ8yKZ-UWmAEFg/live",
            ),
            live(
                "Euronews English",
                "https://www.youtube.com/channel/UCSrZ3UV4jOidv8ppoVuvW9Q/live",
            ),
        ],
    },
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_preset_is_acceptable_to_the_validators() {
        for country in COUNTRIES {
            for preset in country.feeds {
                assert!(super::super::wall_news::feed_url(preset.url).is_some(), "{}", preset.url);
            }
            for preset in country.videos {
                assert!(
                    super::super::wall_news::embed_url(preset.url).is_some(),
                    "{} ({})",
                    preset.label,
                    preset.url
                );
            }
        }
    }
}
