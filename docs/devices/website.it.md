# Salute e SEO del sito web

Monitora i siti web in massa, un dispositivo per sito: sii avvisato quando un sito **si rompe**
(errore, irraggiungibile, certificato non valido, loop di reindirizzamento), quando la sua pagina **diventa vuota**, quando **esce dai motori di ricerca** (`noindex`, `robots.txt`
che blocca tutto) e quando i metadati che Google mostra **cambiano** (titolo,
descrizione, URL canonico, indirizzo finale). Ogni sito è anche ordinato per la
piattaforma su cui viene eseguito (WordPress, Shopify, Wix, Next.js…).

Per il contenuto di una pagina piuttosto che la sua salute, vedi
[Modifiche al sito web](webchange.md).

## Configurarlo

1. Nell'indirizzo, incolla la pagina da controllare, di solito la home page, con o senza https://. I reindirizzamenti vengono seguiti: da http a https e da example.com a www.example.com sono normali, solo un loop di reindirizzamento o uno spostamento a un altro indirizzo viene segnalato.
2. Facoltativamente compila "Testo previsto" con una frase che la pagina deve sempre mostrare, e regola "Pagina vuota sotto" se la pagina è volutamente breve.
3. Il primo controllo registra il riferimento: indirizzo finale, titolo, descrizione, URL canonico, regole di indicizzazione. Una modifica viene segnalata solo una volta che due controlli di fila l'hanno vista, quindi i test A/B e le cache instabili rimangono silenziosi.
4. Per aggiungere molti siti contemporaneamente, importa un file CSV dall'elenco dei dispositivi: ogni sito viene archiviato sotto `Websites/<platform>` (WordPress, Shopify…) a meno che non dai una cartella.

!!! warning
    Solo quello che il server invia viene letto: una pagina disegnata interamente da JavaScript appare vuota qui. Abbassa "Pagina vuota sotto" a 0 per un sito così.

## Cosa fa ogni controllo

Ad ogni intervallo di polling (60 s per impostazione predefinita), la pagina viene letta una volta:

- **I reindirizzamenti vengono seguiti** uno per uno, fino a **Reindirizzamenti max** (10 per
  impostazione predefinita), e la catena viene registrata: ogni indirizzo e il suo stato. Andare da
  `http://` a `https://`, o da `example.com` a `www.example.com`, è quello
  che fanno la maggior parte dei siti e non è mai un errore. Un reindirizzamento che torna a un indirizzo
  già visto è un **loop**, e più reindirizzamenti che consentito è un errore anche:
  entrambi rendono il controllo non riuscito con la catena nel messaggio, come un sito che non
  risponde.
- **La risposta finale**: il suo stato HTTP, il tempo totale (reindirizzamenti inclusi)
  e il certificato TLS, verificato come fa un browser. Un certificato rifiutato
  rende il controllo non riuscito.
- **Il testo visibile** della pagina, come in [Modifiche al sito web](webchange.md):
  script, stili e blocchi nascosti vengono lasciati fuori. Sotto **Pagina vuota sotto**
  caratteri (20 per impostazione predefinita, in modo che le pagine brevi legittime passino) la pagina conta come vuota. Con
  **Testo previsto**, quella frase deve apparire al suo interno (maiuscole/minuscole e spaziatura ignorate).
- **I metadati SEO** della pagina finale: `<title>`, meta descrizione,
  URL canonico, tag meta robot e intestazioni `X-Robots-Tag` (`noindex`,
  `nofollow`, `none`), `og:title`, `og:description`, `<html lang>`, il numero
  di `<h1>` e di alternative `hreflang`.
- **Una volta all'ora**, `robots.txt` del sito finale: vieta a Googlebot
  (o `*`) di eseguire il crawl di `/`? E la sitemap che annuncia, o `/sitemap.xml`: risponde
  e legge come una sitemap?

Lo User-Agent è `DumbMonit/<version>` a meno che **User agent** non dica diversamente.

## Modifiche, confermate due volte

L'indirizzo finale e i metadati vengono confrontati a un riferimento. Un valore diverso
visto una volta è solo **pending**; diventa una modifica quando il prossimo controllo
lo vede di nuovo. Una pagina servita in due versioni (un test A/B), una cache che
esita o una risposta troncata rimangono quindi silenziose. I valori vengono confrontati con
spaziatura e maiuscole ignorate; gli indirizzi ignorano lo schema, un `www.` iniziale, uno
finale `/` e, con **Ignora stringhe di query** (attivo per impostazione predefinita), parametri URL
come `?utm_source=…`.

Una modifica confermata viene conservata nella cronologia del dispositivo (campo, prima, dopo,
data), e il suo misuratore (`dumbmonit_website_title_changed`…) rimane a 1 per 24
ore. Il primo controllo registra solo il riferimento. Cambiare l'indirizzo o
**Ignora stringhe di query** avvia un nuovo riferimento invece di segnalare una modifica.

## Opzioni

| Opzione | Impostazione predefinita | Significato |
|---|---|---|
| Testo previsto | vuoto | Una frase che la pagina deve mostrare. |
| Pagina vuota sotto (caratteri) | 20 | Il testo visibile più breve di questo è una pagina vuota; 0 disattiva il controllo. |
| Reindirizzamenti max | 10 | Reindirizzamenti seguiti al massimo (0 a 30). |
| User agent | `DumbMonit/<version>` | Inviato invece del proprio di DumbMonit. |
| Ignora stringhe di query | attivo | I parametri URL non contano quando si confrontano gli indirizzi. |
| Consenti target loopback e link-local | disattivo | Per un sito servito dall'host DumbMonit stesso. |

## Rilevamento della piattaforma

La piattaforma proviene da, in ordine: il tag `<meta name="generator">`, intestazioni
specifiche a un host o motore (`x-shopify-*`, `x-wix-*`, `x-drupal-cache`,
`x-powered-by: Next.js`…), tracce nel markup (`/wp-content/`,
`cdn.shopify.com`, `data-wf-site`, `__NEXT_DATA__`, `/_nuxt/`…) e finalmente lo
host stesso (`*.myshopify.com`, `*.wixsite.com`…). Riconosciuti: WordPress,
Shopify, Wix, Squarespace, Webflow, Ghost, Drupal, Joomla, PrestaShop, Magento,
Next.js, Nuxt, Gatsby, Hugo, Jekyll; qualsiasi altro è **Other**. La piattaforma
è l'etichetta `platform` di `dumbmonit_website_info`.

## Importa un elenco di siti (CSV)

Dall'elenco dei dispositivi o dalla pagina aggiungi dispositivo, **Importa siti web (CSV)** prende un
file o testo incollato con una riga di intestazione e fino a 1000 righe:

| Colonna | Obbligatoria | Significato |
|---|---|---|
| `url` | sì | Indirizzo del sito. |
| `name` | no | Nome del dispositivo; l'host per impostazione predefinita. |
| `folder` | no | Cartella del dispositivo; vuota: `Websites/<platform>`, rilevata leggendo il sito una volta. |
| `tags` | no | `key=value;key=value`. |
| `interval` | no | Intervallo di polling. |
| `expect_text` | no | Stesso dell'opzione. |

```csv
url,name,folder,tags,interval,expect_text
https://example.com,Example,,client=acme,60s,Example Inc.
https://shop.example.org,Shop,Customers/Acme,,300s,Add to cart
```

Un'anteprima mostra ogni riga e i suoi errori prima che qualsiasi cosa venga creata; un sito
già monitorato (stesso indirizzo finale) viene saltato.

## Metriche

| Metrica | Significato |
|---|---|
| `dumbmonit_website_up` | 1 quando la risposta finale è un successo (2xx o 3xx), 0 altrimenti. |
| `dumbmonit_website_status_code` | Stato HTTP della risposta finale. |
| `dumbmonit_website_response_seconds` | Tempo totale, reindirizzamenti inclusi. |
| `dumbmonit_website_redirect_hops` | Reindirizzamenti seguiti. |
| `dumbmonit_website_info{platform}` | Sempre 1; l'etichetta porta la piattaforma. |
| `dumbmonit_website_text_chars` | Caratteri di testo visibile (risposte 2xx). |
| `dumbmonit_website_blank` | 1 quando quel testo è sotto la soglia. |
| `dumbmonit_website_expect_text_missing` | 1 quando il testo previsto è mancante (opzione impostata). |
| `dumbmonit_website_noindex` / `dumbmonit_website_nofollow` | Direttive di indicizzazione. |
| `dumbmonit_website_h1_count` / `dumbmonit_website_hreflang_count` | Intestazioni `<h1>`, alternative `hreflang`. |
| `dumbmonit_website_title_changed`, `…_description_changed`, `…_canonical_changed`, `…_final_url_changed` | 1 per 24 ore dopo una modifica confermata. |
| `dumbmonit_website_robots_blocks_all` | 1 quando `robots.txt` vieta tutto a Googlebot. |
| `dumbmonit_website_sitemap_ok` | 1 quando una sitemap risponde e legge. |

## Allarmi integrati

| Allarme | Quando | Severità |
|---|---|---|
| Dispositivo irraggiungibile | Nessuna risposta, certificato rifiutato, loop di reindirizzamento o troppi reindirizzamenti | critico |
| Sito web inattivo | La risposta finale è un errore 4xx o 5xx, per 3 minuti | critico |
| Pagina sito web vuota | Testo visibile sotto la soglia, per 3 minuti | critico |
| Testo previsto del sito web mancante | Il testo previsto è mancante, per 3 minuti | avvertimento |
| Sito web non indicizzabile | `noindex` sulla pagina, per 10 minuti | critico |
| Sito web bloccato da robots.txt | `robots.txt` vieta tutto, per 10 minuti | critico |
| URL canonico del sito web modificato | Modifica confermata | avvertimento |
| Indirizzo del sito web modificato | L'host finale o il percorso è cambiato, confermato | avvertimento |
| Titolo o descrizione del sito web modificati | Modifica confermata | info |
| Sito web lento | Più di 5 secondi, per 10 minuti | avvertimento |

Il controllo viene eseguito sul server DumbMonit stesso: non può passare attraverso un agente
relè.
