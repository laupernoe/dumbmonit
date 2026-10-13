# Banner per il sito web

Mostra un banner sul tuo sito web quando dei servizi non funzionano ("2 servizi
non funzionano"), alimentato da una [pagina di stato](../using/status-pages.md)
pubblicata. Quando tutto va bene il banner resta nascosto, a meno che tu non
chieda di mostrarlo ("Tutti i sistemi operativi").

Tre modi per aggiungerlo, dal più semplice al più controllato. I file pronti da
copiare si trovano nella cartella `integrations/` del repository.

## Snippet HTML

Incolla questo prima di `</body>`, con l'indirizzo del tuo server e quello della pagina:

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

Lo script non ha dipendenze, disegna all'interno di uno Shadow DOM in modo che i
tuoi stili non lo tocchino, segue la preferenza chiaro o scuro del visitatore e
può essere chiuso (fino alla chiusura della scheda del browser). Legge il JSON
pubblico della pagina, che il server consente a qualsiasi sito web di scaricare
(`Access-Control-Allow-Origin: *`, nessun cookie). Solo il documento pubblico
della pagina di stato viene aperto in questo modo; il resto dell'API mantiene le
sue regole.

## WordPress

Copia `integrations/wordpress/dumbmonit-status-banner` in
`wp-content/plugins/`, attivalo, poi apri **Impostazioni > DumbMonit Banner**
e inserisci l'URL della pagina di stato
(`https://monit.example.com/api/public/status/my-page`), la posizione e la
modalità di visualizzazione. Il plugin scarica il JSON lato server, lo mantiene
in cache per un minuto ed esegue l'escape di tutto ciò che stampa.

Il server DumbMonit può trovarsi a un indirizzo privato (LAN): il plugin
richiede sempre e soltanto l'URL esatto che hai salvato (verificato rispetto al
modello `/api/public/status/<address>`, reindirizzamenti disabilitati). Se
l'indirizzo non è valido, la pagina delle impostazioni mostra un errore e
mantiene il valore precedente. Se la pagina di stato non può essere scaricata,
viene mostrato un avviso nell'amministrazione di WordPress (e al salvataggio),
invece di fallire in silenzio.

Il testo del banner è traducibile (text domain `dumbmonit-status-banner`, modello
in `languages/`); segue la lingua del sito WordPress. Il JSON pubblico dello
stato non contiene alcuna lingua, quindi non può sceglierne una.

## Include PHP

Per qualsiasi sito PHP, senza framework:

```php
require __DIR__ . '/banner.php';
dumbmonit_banner('https://monit.example.com/api/public/status/my-page',
    ['position' => 'top', 'show' => 'issues', 'ttl' => 60]);
```

L'helper accetta solo URL `http` e `https`, usa un timeout di 3 secondi, non
segue i reindirizzamenti, mantiene una cache su file nella directory temporanea
e non stampa nulla se il server non è raggiungibile (l'errore viene scritto nel
log degli errori di PHP). I testi sono in inglese per impostazione predefinita;
passa `'lang' => 'fr'` o un array `'strings'` per modificarli. Un indirizzo
privato (LAN) funziona. Vedi `integrations/php/example.php`.
