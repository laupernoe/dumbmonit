# Banner per il sito web

Mostra un banner sul tuo sito web quando i servizi sono inattivi ("2 servizi
sono inattivi"), alimentato da una [pagina di stato](../using/status-pages.md)
pubblicata. Quando tutto va bene il banner resta nascosto, a meno che tu non
chieda di mostrarlo comunque ("All systems operational").

Tre modi per aggiungerlo, dal più semplice al più controllato. File pronti da
copiare si trovano nella cartella `integrations/` del repository.

## Snippet HTML

Incolla questo prima di `</body>`, con l'indirizzo del tuo server e
l'indirizzo della pagina:

```html
<script async
  src="https://monit.example.com/api/public/status/my-page/banner.js"
  data-position="top" data-show="issues" data-lang="en"></script>
```

| Attributo | Valori | Predefinito |
|---|---|---|
| `data-position` | `top`, `bottom` | `top` |
| `data-show` | `issues` (solo quando qualcosa non va), `always` | `issues` |
| `data-lang` | `en`, `fr`, `de`, `es`, `it`, `pt`, `pt-BR`, `ru`, `zh-Hans` | lingua della pagina, altrimenti `en` |

Lo script non ha dipendenze, viene disegnato dentro uno Shadow DOM così i tuoi
stili non lo toccano, segue la preferenza chiaro/scuro del visitatore e può
essere chiuso (finché la scheda del browser non viene chiusa). Legge il JSON
pubblico della pagina, che il server consente a qualsiasi sito web di
recuperare (`Access-Control-Allow-Origin: *`, senza cookie). Solo il documento
pubblico della pagina di stato viene aperto in questo modo; il resto dell'API
mantiene le proprie regole.

## WordPress

Copia `integrations/wordpress/dumbmonit-status-banner` in
`wp-content/plugins/`, attivalo, poi apri **Settings > DumbMonit Banner** e
inserisci l'URL della pagina di stato
(`https://monit.example.com/api/public/status/my-page`), la posizione e la
modalità di visualizzazione. Il plugin recupera il JSON sul server, lo mette
in cache per un minuto ed esegue l'escaping di tutto ciò che stampa. WordPress
rifiuta le richieste verso indirizzi privati, quindi il server DumbMonit deve
essere raggiungibile su un indirizzo pubblico.

## Include PHP

Per qualsiasi sito PHP, senza framework:

```php
require __DIR__ . '/banner.php';
dumbmonit_banner('https://monit.example.com/api/public/status/my-page',
    ['position' => 'top', 'show' => 'issues', 'ttl' => 60]);
```

L'helper accetta solo URL `http` e `https`, usa un timeout di 3 secondi, non
segue i redirect, mantiene una cache su file nella directory temporanea e non
stampa nulla se il server non è raggiungibile. Vedi
`integrations/php/example.php`.
