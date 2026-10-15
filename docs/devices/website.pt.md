# Saúde e SEO de sites {#website-health-seo}

Vigie sites em massa, um monitor por site: seja avisado quando um site
**avaria** (erro, inacessível, certificado inválido, ciclo de redirecionamento),
quando a sua página **fica em branco**, quando **desaparece dos motores de
pesquisa** (`noindex`, `robots.txt` a bloquear tudo) e quando os metadados que
o Google mostra **mudam** (título, descrição, URL canónico, endereço final).
Cada site é também classificado pela plataforma em que corre (WordPress,
Shopify, Wix, Next.js…).

Para o conteúdo de uma página em vez da sua saúde, veja
[Alterações em sites](webchange.md).

## Configurar {#set-it-up}

1. No endereço, cole a página a verificar, normalmente a página inicial, com ou sem https://. Os redirecionamentos são seguidos: de http para https e de example.com para www.example.com são normais, só um ciclo de redirecionamento ou uma mudança para outro endereço é reportado.
2. Opcionalmente preencha «Texto esperado» com uma frase que a página deve mostrar sempre, e ajuste «Página em branco abaixo de» se a página for curta de propósito.
3. A primeira verificação regista a referência: endereço final, título, descrição, URL canónico, regras de indexação. São comparados de novo a cada 6 horas por predefinição («Verificar alterações a cada»), enquanto o estado ativo/inativo é verificado em cada intervalo de sondagem. Uma alteração só é reportada depois de duas verificações seguidas a terem visto, para que testes A/B e caches instáveis fiquem em silêncio.
4. Para adicionar muitos sites de uma vez, importe um ficheiro CSV a partir da lista de monitores: cada site é arquivado em `Websites/<platform>` (WordPress, Shopify…) a menos que indique uma pasta.

!!! warning
    Só é lido o que o servidor envia: uma página desenhada inteiramente por JavaScript parece em branco aqui. Baixe «Página em branco abaixo de» para 0 para um site assim.

## O que faz cada verificação {#what-each-check-does}

A cada intervalo de sondagem (60 s por predefinição), a página é lida uma vez:

- **Os redirecionamentos são seguidos** um a um, até **Redirecionamentos
  máximos** (10 por predefinição), e a cadeia é registada: cada endereço e o
  seu estado. Passar de `http://` para `https://`, ou de `example.com` para
  `www.example.com`, é o que a maioria dos sites faz e nunca é um erro. Um
  redirecionamento que volta a um endereço já visto é um **ciclo**, e mais
  redirecionamentos do que o permitido também é um erro: ambos fazem a
  verificação falhar com a cadeia na mensagem, como um site que não responde.
- **A resposta final**: o seu estado HTTP, o tempo total (redirecionamentos
  incluídos) e o certificado TLS, verificado como um navegador o faz. Um
  certificado recusado faz a verificação falhar.
- **O texto visível** da página, como em [Alterações em sites](webchange.md):
  scripts, estilos e blocos ocultos ficam de fora. Abaixo de **Página em branco
  abaixo de** caracteres (20 por predefinição, para que páginas legítimas
  curtas passem) a página conta como em branco. Com **Texto esperado**, essa
  frase tem de aparecer nele (maiúsculas e espaçamento ignorados).
- **Os metadados de SEO** da página final: `<title>`, meta description, URL
  canónico, meta tags robots e cabeçalhos `X-Robots-Tag` (`noindex`,
  `nofollow`, `none`), `og:title`, `og:description`, `<html lang>`, o número de
  `<h1>` e de alternativas `hreflang`. As diretivas de indexação e as contagens
  são reportadas em cada verificação; comparar os metadados com a referência só
  acontece nas verificações de alterações, veja [abaixo](#changes-confirmed-twice).
- **Nas verificações de alterações, e no máximo uma vez por hora**, o
  `robots.txt` do site final: proíbe o Googlebot (ou `*`) de rastrear `/`? E o
  sitemap que anuncia, ou `/sitemap.xml`: responde e lê-se como um sitemap?

O User-Agent é `DumbMonit/<version>` a menos que **User agent** diga outra
coisa.

## Alterações, confirmadas duas vezes {#changes-confirmed-twice}

O endereço final e os metadados são comparados com uma referência, mas não em
cada verificação: apenas nas **verificações de alterações**, a cada **Verificar
alterações a cada** minutos (360, ou seja 6 horas, por predefinição;
`DUMBMONIT_WEBSITE_CHANGE_CHECK_MINUTES` define a predefinição de todo o
servidor, 0 compara em cada verificação). O estado ativo/inativo, a página em
branco e o texto esperado continuam a ser lidos em cada intervalo de sondagem.

Um valor diferente visto numa verificação de alterações fica apenas
**pendente**; a verificação imediatamente seguinte (um intervalo de sondagem
depois, não 6 horas depois) volta a olhar para ele, e torna-se uma alteração se
essa verificação o vir também. Uma alteração é portanto reportada no máximo um
período de alterações mais um intervalo de sondagem depois de aparecer. Uma
página servida em duas versões (um teste A/B), uma cache que hesita ou uma
resposta truncada ficam portanto em silêncio. Os valores são comparados
ignorando espaçamento e maiúsculas; os endereços ignoram o esquema, um `www.`
inicial, uma `/` final e, com **Ignorar query strings** (ativado por
predefinição), parâmetros de URL como `?utm_source=…`.

Uma alteração confirmada é guardada no histórico do monitor (campo, antes,
depois, data), e o seu indicador (`dumbmonit_website_title_changed`…) fica a 1
durante 24 horas. A primeira verificação apenas regista a referência. Mudar o
endereço ou **Ignorar query strings** inicia uma nova referência em vez de
reportar uma alteração.

## Opções {#options}

| Opção | Predefinição | Significado |
|---|---|---|
| Texto esperado | vazio | Uma frase que a página deve mostrar. |
| Página em branco abaixo de (caracteres) | 20 | Um texto visível mais curto do que isto é uma página em branco; 0 desativa a verificação. |
| Redirecionamentos máximos | 10 | Redirecionamentos seguidos no máximo (0 a 30). |
| User agent | `DumbMonit/<version>` | Enviado em vez do próprio do DumbMonit. |
| Ignorar query strings | ativado | Os parâmetros de URL não contam ao comparar endereços. |
| Verificar alterações a cada (minutos) | 360 | Com que frequência os metadados, o endereço final e o `robots.txt` são comparados, 0 a 10080; 0 significa em cada verificação. Vazio: a predefinição do servidor (`DUMBMONIT_WEBSITE_CHANGE_CHECK_MINUTES`). |
| Esperar antes do pedido seguinte a este domínio (segundos) | 2 | Intervalo mínimo após a verificação deste site antes de outra verificação no mesmo domínio, 0 a 3600. Vazio: a predefinição do servidor (`DUMBMONIT_WEBSITE_HOST_SPACING_SECS`). |
| Permitir alvos loopback e link-local | desativado | Para um site servido pelo próprio anfitrião do DumbMonit. |

## Distribuir centenas de sites no tempo {#pacing-hundreds-of-sites}

As verificações nunca partem numa rajada, para que uma firewall ou CDN à frente
dos sites não confunda o DumbMonit com um ataque:

- **Espalhadas pelo intervalo**: cada site arranca no seu próprio desfasamento
  fixo dentro do seu intervalo de sondagem, pelo que sites adicionados juntos (uma
  importação CSV) não disparam no mesmo segundo.
- **Limite global**: no máximo `DUMBMONIT_WEBSITE_MAX_CONCURRENT` verificações de
  sites em curso (10 por predefinição), além de `DUMBMONIT_MAX_CONCURRENT_PROBES`.
- **Por domínio**: no máximo `DUMBMONIT_WEBSITE_MAX_PER_HOST` verificações em
  curso para um domínio (1 por predefinição), e pelo menos
  `DUMBMONIT_WEBSITE_HOST_SPACING_SECS` segundos entre duas delas (2 por
  predefinição, ou o **Esperar antes do pedido seguinte a este domínio** do
  site). O domínio é o nome registável: `www.example.com`, `shop.example.com` e
  `example.com` partilham um, `example.co.uk` é um; os subdomínios de clientes
  de plataformas de alojamento (`*.myshopify.com`, `*.wordpress.com`,
  `*.github.io`, `*.netlify.app`…) contam cada um como o seu próprio.

Um site cuja verificação anterior ainda espera a sua vez salta uma ronda em vez de
entrar duas vezes na fila. Muitos sites num só domínio com um intervalo de
sondagem curto são portanto verificados com menos frequência do que o pedido:
aumente o intervalo, ou os limites por domínio, se a lista de monitores os
mostrar atrasados. Veja [Configuração](../reference/configuration.md#server).

## Deteção da plataforma {#platform-detection}

A plataforma vem, por ordem: da tag `<meta name="generator">`, de cabeçalhos
específicos de um alojamento ou motor (`x-shopify-*`, `x-wix-*`,
`x-drupal-cache`, `x-powered-by: Next.js`…), de vestígios no markup
(`/wp-content/`, `cdn.shopify.com`, `data-wf-site`, `__NEXT_DATA__`,
`/_nuxt/`…) e por fim do próprio anfitrião (`*.myshopify.com`,
`*.wixsite.com`…). Reconhecidas: WordPress, Shopify, Wix, Squarespace,
Webflow, Ghost, Drupal, Joomla, PrestaShop, Magento, Next.js, Nuxt, Gatsby,
Hugo, Jekyll; tudo o resto é **Other**. A plataforma é a etiqueta `platform`
de `dumbmonit_website_info`.

## Importar uma lista de sites (CSV) {#import-a-list-of-sites-csv}

A partir da lista de monitores ou da página de adicionar monitor,
**Importar sites (CSV)** aceita um ficheiro ou texto colado com uma linha de
cabeçalho e até 1000 linhas:

| Coluna | Obrigatória | Significado |
|---|---|---|
| `url` | sim | Endereço do site. |
| `name` | não | Nome do monitor; o anfitrião por predefinição. |
| `folder` | não | Pasta do monitor; vazio: `Websites/<platform>`, detetada lendo o site uma vez. |
| `tags` | não | `key=value;key=value`. |
| `interval` | não | Intervalo de sondagem. |
| `expect_text` | não | O mesmo que a opção. |

```csv
url,name,folder,tags,interval,expect_text
https://example.com,Example,,client=acme,60s,Example Inc.
https://shop.example.org,Shop,Customers/Acme,,300s,Add to cart
```

Uma pré-visualização mostra cada linha e os seus erros antes de se criar
qualquer coisa; um site já monitorizado (mesmo endereço final) é ignorado.

## Métricas {#metrics}

| Métrica | Significado |
|---|---|
| `dumbmonit_website_up` | 1 quando a resposta final é um sucesso (2xx ou 3xx), 0 caso contrário. |
| `dumbmonit_website_status_code` | Estado HTTP da resposta final. |
| `dumbmonit_website_response_seconds` | Tempo total, redirecionamentos incluídos. |
| `dumbmonit_website_redirect_hops` | Redirecionamentos seguidos. |
| `dumbmonit_website_https` | 1 quando o endereço final é https. |
| `dumbmonit_website_security_header{header}` | 0/1 por etiqueta `header` (apenas https): HSTS, nosniff, CSP, X-Frame-Options, frame-ancestors. |
| `dumbmonit_website_info{platform}` | Sempre 1; a etiqueta traz a plataforma. |
| `dumbmonit_website_text_chars` | Caracteres de texto visível (respostas 2xx). |
| `dumbmonit_website_blank` | 1 quando esse texto está abaixo do limiar. |
| `dumbmonit_website_expect_text_missing` | 1 quando o texto esperado está em falta (opção definida). |
| `dumbmonit_website_noindex` / `dumbmonit_website_nofollow` | Diretivas de indexação. |
| `dumbmonit_website_h1_count` / `dumbmonit_website_hreflang_count` | Títulos `<h1>`, alternativas `hreflang`. |
| `dumbmonit_website_title_changed`, `…_description_changed`, `…_canonical_changed`, `…_final_url_changed` | 1 durante 24 horas após uma alteração confirmada. |
| `dumbmonit_website_robots_blocks_all` | 1 quando o `robots.txt` proíbe tudo ao Googlebot. |
| `dumbmonit_website_sitemap_ok` | 1 quando um sitemap responde e se lê. |

## Alertas incorporados {#built-in-alerts}

| Alerta | Quando | Gravidade |
|---|---|---|
| Monitor unreachable | Sem resposta, certificado recusado, ciclo de redirecionamento ou demasiados redirecionamentos | critical |
| Website down | A resposta final é um erro 4xx ou 5xx, durante 3 minutos | critical |
| Website page blank | Texto visível abaixo do limiar, durante 3 minutos | critical |
| Website expected text missing | O texto esperado está em falta, durante 3 minutos | warning |
| Website not indexable | `noindex` na página, durante 10 minutos | critical |
| Website blocked by robots.txt | O `robots.txt` proíbe tudo, durante 10 minutos | critical |
| Website canonical URL changed | Alteração confirmada | warning |
| Website address changed | O anfitrião ou caminho finais mudaram, confirmado | warning |
| Website title or description changed | Alteração confirmada | info |
| Website slow | Mais de 5 segundos, durante 10 minutos | warning |

A verificação corre no próprio servidor DumbMonit: não pode passar por um agente
de retransmissão.
