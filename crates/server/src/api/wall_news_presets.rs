//! Sources d'actualités proposées par pays pour le mode mur.
//!
//! Des données, pas de la logique : chaque flux a été relu à la main (le
//! 14/10/2026, les flux tech et IA le 19/10/2026 : HTTP 200, document RSS/Atom
//! lisible, articles présents). Un flux qui cesserait de répondre n'est pas un
//! bogue du serveur : l'administrateur voit l'erreur dans ses réglages et peut
//! en saisir un autre (« flux personnalisé »).
//!
//! Deux groupes thématiques, `TECH` et `AI` (sources en anglais, publiques, sans
//! clé), s'ajoutent aux pays ; les pays reçoivent aussi leurs sites tech.

use serde::Serialize;

#[derive(Serialize)]
pub struct FeedPreset {
    pub label: &'static str,
    pub url: &'static str,
}

#[derive(Serialize)]
pub struct Country {
    /// Code ISO 3166-1 alpha-2, `TECH` ou `AI` pour les groupes thématiques,
    /// `INT` pour les sources internationales.
    pub code: &'static str,
    pub feeds: &'static [FeedPreset],
}

const fn feed(label: &'static str, url: &'static str) -> FeedPreset {
    FeedPreset { label, url }
}

pub static COUNTRIES: &[Country] = &[
    Country {
        code: "FR",
        feeds: &[
            feed("BFM TV", "https://www.bfmtv.com/rss/news-24-7/"),
            feed("franceinfo", "https://www.francetvinfo.fr/titres.rss"),
            feed("Le Monde", "https://www.lemonde.fr/rss/une.xml"),
            feed("France 24", "https://www.france24.com/fr/rss"),
            feed("Korben", "https://korben.info/feed"),
            feed("Next", "https://next.ink/feed/"),
            feed("Numerama", "https://www.numerama.com/feed/"),
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
    },
    Country {
        code: "BE",
        feeds: &[
            feed("RTBF", "https://rss.rtbf.be/article/rss/highlight_rtbf_info.xml"),
            feed("VRT NWS", "https://www.vrt.be/vrtnws/nl.rss.articles.xml"),
            feed("La DH", "https://www.dhnet.be/arc/outboundfeeds/rss/?outputType=xml"),
        ],
    },
    Country {
        code: "DE",
        feeds: &[
            feed("Tagesschau", "https://www.tagesschau.de/index~rss2.xml"),
            feed("ZDFheute", "https://www.zdf.de/rss/zdf/nachrichten"),
            feed("Spiegel", "https://www.spiegel.de/schlagzeilen/index.rss"),
            feed("Deutsche Welle", "https://rss.dw.com/rdf/rss-de-all"),
            feed("heise online", "https://www.heise.de/rss/heise-atom.xml"),
            feed("Golem", "https://rss.golem.de/rss.php?feed=RSS2.0"),
        ],
    },
    Country {
        code: "GB",
        feeds: &[
            feed("BBC News", "https://feeds.bbci.co.uk/news/rss.xml"),
            feed("The Guardian", "https://www.theguardian.com/uk/rss"),
        ],
    },
    Country {
        code: "US",
        feeds: &[
            feed("NPR", "https://feeds.npr.org/1001/rss.xml"),
            feed("PBS NewsHour", "https://www.pbs.org/newshour/feeds/rss/headlines"),
            feed("CBS News", "https://www.cbsnews.com/latest/rss/main"),
            feed("The New York Times", "https://rss.nytimes.com/services/xml/rss/nyt/HomePage.xml"),
        ],
    },
    Country {
        code: "CA",
        feeds: &[
            feed("Radio-Canada", "https://ici.radio-canada.ca/rss/4159"),
            feed("CBC News", "https://www.cbc.ca/webfeed/rss/rss-topstories"),
        ],
    },
    Country {
        code: "ES",
        feeds: &[
            feed("RTVE", "https://www.rtve.es/rss/temas_noticias.xml"),
            feed("20minutos", "https://www.20minutos.es/rss/"),
            feed("El País", "https://feeds.elpais.com/mrss-s/pages/ep/site/elpais.com/portada"),
            feed("El Mundo", "https://e00-elmundo.uecdn.es/elmundo/rss/portada.xml"),
            feed("Xataka", "https://www.xataka.com/feedburner.xml"),
        ],
    },
    Country {
        code: "IT",
        feeds: &[
            feed("ANSA", "https://www.ansa.it/sito/ansait_rss.xml"),
            feed("RaiNews", "https://www.rainews.it/rss/tutti"),
            feed("Punto Informatico", "https://www.punto-informatico.it/feed/"),
        ],
    },
    Country {
        code: "PT",
        feeds: &[
            feed("RTP Notícias", "https://www.rtp.pt/noticias/rss"),
            feed("Público", "https://feeds.feedburner.com/PublicoRSS"),
        ],
    },
    Country {
        code: "BR",
        feeds: &[
            feed("G1", "https://g1.globo.com/rss/g1/"),
            feed("Folha de S.Paulo", "https://feeds.folha.uol.com.br/emcimadahora/rss091.xml"),
            feed("Tecnoblog", "https://tecnoblog.net/feed/"),
        ],
    },
    Country {
        code: "RU",
        feeds: &[
            feed("Interfax", "https://www.interfax.ru/rss.asp"),
            feed("Meduza", "https://meduza.io/rss/all"),
            feed("Kommersant", "https://www.kommersant.ru/RSS/main.xml"),
            feed("Habr", "https://habr.com/ru/rss/all/all/"),
        ],
    },
    Country {
        code: "CN",
        feeds: &[
            feed("CGTN", "https://www.cgtn.com/subscribe/rss/section/world.xml"),
            feed("South China Morning Post", "https://www.scmp.com/rss/91/feed"),
            feed("Xinhua", "http://www.xinhuanet.com/english/rss/worldrss.xml"),
        ],
    },
    // Technologie, en anglais : des sites publics qui tiennent un flux depuis des années.
    Country {
        code: "TECH",
        feeds: &[
            feed("Hacker News", "https://hnrss.org/frontpage"),
            feed("Ars Technica", "https://feeds.arstechnica.com/arstechnica/index"),
            feed("The Verge", "https://www.theverge.com/rss/index.xml"),
            feed("TechCrunch", "https://techcrunch.com/feed/"),
            feed("Wired", "https://www.wired.com/feed/rss"),
            feed("The Register", "https://www.theregister.com/headlines.atom"),
            feed("Engadget", "https://www.engadget.com/rss.xml"),
            feed("Lobsters", "https://lobste.rs/rss"),
        ],
    },
    // Intelligence artificielle, en anglais : laboratoires, blogs et presse spécialisée.
    Country {
        code: "AI",
        feeds: &[
            feed("OpenAI", "https://openai.com/news/rss.xml"),
            feed("Google AI", "https://blog.google/technology/ai/rss/"),
            feed("Hugging Face", "https://huggingface.co/blog/feed.xml"),
            feed(
                "MIT Technology Review",
                "https://www.technologyreview.com/topic/artificial-intelligence/feed",
            ),
            feed("Simon Willison", "https://simonwillison.net/atom/everything/"),
            feed("BAIR", "https://bair.berkeley.edu/blog/feed.xml"),
            feed("NVIDIA", "https://blogs.nvidia.com/feed/"),
            feed("AWS Machine Learning", "https://aws.amazon.com/blogs/machine-learning/feed/"),
        ],
    },
    Country {
        code: "INT",
        feeds: &[
            feed("Deutsche Welle", "https://rss.dw.com/rdf/rss-en-all"),
            feed("France 24", "https://www.france24.com/en/rss"),
            feed("BBC News (world)", "https://feeds.bbci.co.uk/news/world/rss.xml"),
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
        }
    }
}
