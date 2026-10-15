# Saúde e SEO do site {#website-health--seo}

Monitore sites em massa, um monitor por site: seja informado quando um site **quebra**
(erro, inacessível, certificado inválido, loop de redirecionamento), quando sua página **fica
em branco**, quando ela **desaparece dos mecanismos de pesquisa** (`noindex`, `robots.txt`
bloqueando tudo) e quando os metadados que o Google mostra **mudam** (título,
descrição, URL canônica, endereço final). Cada site também é classificado pela
plataforma em que é executado (WordPress, Shopify, Wix, Next.js…).

Para o conteúdo de uma página em vez de sua saúde, consulte
[Mudanças do site](webchange.md).

## Configurar {#set-it-up}

1. No endereço, cole a página para verificar, geralmente a página inicial, com ou sem https://. Os redirecionamentos são seguidos: http para https e example.com para www.example.com são normais, apenas um loop de redirecionamento ou uma mudança para outro endereço é reportado.
2. Opcionalmente, preencha "Texto esperado" com uma frase que a página deve sempre mostrar e ajuste "Página em branco abaixo" se a página for curta propositalmente.
3. A primeira verificação registra a referência: endereço final, título, descrição, URL canônica, regras de indexação. Eles são comparados novamente a cada 6 horas por padrão ("Verificar mudanças a cada"), enquanto up/down é verificado em cada intervalo de polling. Uma mudança é reportada apenas depois que duas verificações consecutivas a viram, portanto testes A/B e caches instáveis ficam silenciosos.
4. Para adicionar muitos sites de uma vez, importe um arquivo CSV da lista de monitores: cada site é classificado em `Websites/<plataforma>` (WordPress, Shopify…) a menos que você especifique uma pasta.

!!! warning
    Apenas o que o servidor envia é lido: uma página desenhada inteiramente por JavaScript parece em branco aqui. Reduza "Página em branco abaixo" para 0 para tal site.

## O que cada verificação faz {#what-each-check-does}

A cada intervalo de polling (60 s por padrão), a página é lida uma vez:

- **Os redirecionamentos são seguidos** um por um, até **Máximo de redirecionamentos** (10 por
  padrão), e a cadeia é registrada: cada endereço e seu status. Ir de
  `http://` para `https://`, ou de `example.com` para `www.example.com`, é o que
  a maioria dos sites faz e nunca é um erro. Um redirecionamento que volta para um endereço
  já visto é um **loop**, e mais redirecionamentos que o permitido também é um erro:
  ambos fazem a verificação falhar com a cadeia na mensagem, como um site que não
  responde.
- **A resposta final**: seu status HTTP, o tempo total (redirecionamentos incluídos)
  e o certificado TLS, verificado como um navegador faz. Um certificado recusado
  faz a verificação falhar.
- **O texto visível** da página, como em [Mudanças do site](webchange.md):
  scripts, estilos e blocos ocultos são deixados de fora. Abaixo de **Página em branco abaixo**
  caracteres (20 por padrão, então páginas legítimas curtas passam) a página conta como em branco. Com
  **Texto esperado**, essa frase deve aparecer nela (maiúscula e espaçamento ignorados).
- **Os metadados de SEO** da página final: `<title>`, meta description,
  URL canônica, meta tags de robots e cabeçalhos `X-Robots-Tag` (`noindex`,
  `nofollow`, `none`), `og:title`, `og:description`, `<html lang>`, o número
  de `<h1>` e de `hreflang` alternados. As diretivas de indexação e contagens
  são reportadas em cada verificação; comparar os metadados com a referência apenas
  acontece em verificações de mudança, veja [abaixo](#changes-confirmed-twice).
- **Em verificações de mudança, e no máximo uma vez por hora**, `robots.txt` do site final:
  ele proíbe Googlebot (ou `*`) de rastrear `/`? E o sitemap que
  anuncia, ou `/sitemap.xml`: ele responde e lê como um sitemap?

O User-Agent é `DumbMonit/<version>` a menos que **User agent** diga o contrário.

## Mudanças, confirmadas duas vezes {#changes-confirmed-twice}

O endereço final e os metadados são comparados a uma referência, mas não em
cada verificação: apenas em **verificações de mudança**, a cada **Verificar mudanças a cada**
minutos (360, ou seja, 6 horas, por padrão; `DUMBMONIT_WEBSITE_CHANGE_CHECK_MINUTES`
define o padrão em todo o servidor, 0 compara em cada verificação). Up/down, página em branco
e texto esperado ainda são lidos em cada intervalo de polling.

Um valor diferente visto em uma verificação de mudança é apenas **pendente**; a próxima
verificação (um intervalo de polling depois, não 6 horas depois) o vê novamente, e
se torna uma mudança se essa verificação a vir também. Uma mudança é, portanto, reportada
no máximo um período de mudança mais um intervalo de polling após aparecer. Uma página servida em duas versões (um teste A/B), um cache que
hesita ou uma resposta truncada, portanto, fica silencioso. Os valores são comparados com
espaçamento e maiúscula ignorados; endereços ignoram o esquema, um `www.` inicial, um
`/` final e, com **Ignorar strings de consulta** (ativado por padrão), parâmetros de URL
como `?utm_source=…`.

Uma mudança confirmada é mantida no histórico do monitor (campo, antes, depois,
data), e seu medidor (`dumbmonit_website_title_changed`…) permanece em 1 por 24
horas. A primeira verificação apenas registra a referência. Alterar o endereço ou
**Ignorar strings de consulta** inicia uma nova referência em vez de relatar uma mudança.

## Opções {#options}

| Opção | Padrão | Significado |
|---|---|---|
| Texto esperado | vazio | Uma frase que a página deve mostrar. |
| Página em branco abaixo (caracteres) | 20 | Texto visível mais curto que isso é uma página em branco; 0 desativa a verificação. |
| Máximo de redirecionamentos | 10 | Redirecionamentos seguidos no máximo (0 a 30). |
| User agent | `DumbMonit/<version>` | Enviado em vez do próprio DumbMonit. |
| Ignorar strings de consulta | ativado | Parâmetros de URL não contam ao comparar endereços. |
| Verificar mudanças a cada (minutos) | 360 | Com que frequência os metadados, o endereço final e `robots.txt` são comparados, 0 a 10080; 0 significa em cada verificação. Vazio: o padrão do servidor (`DUMBMONIT_WEBSITE_CHANGE_CHECK_MINUTES`). |
| Aguardar antes da próxima solicitação para este domínio (segundos) | 2 | Intervalo mínimo após a verificação deste site antes de outra verificação no mesmo domínio, 0 a 3600. Vazio: o padrão do servidor (`DUMBMONIT_WEBSITE_HOST_SPACING_SECS`). |
| Permitir alvos de loopback e link-local | desativado | Para um site servido pelo host DumbMonit. |

## Centenas de sites em ritmo {#pacing-hundreds-of-sites}

As verificações nunca saem em um burst, portanto um firewall ou CDN em frente aos sites
não confunde o DumbMonit com um ataque:

- **Espalhado ao longo do intervalo**: cada site começa em seu próprio offset fixo
  dentro de seu intervalo de polling, portanto sites adicionados juntos (uma importação CSV) não
  são acionados no mesmo segundo.
- **Limite global**: no máximo `DUMBMONIT_WEBSITE_MAX_CONCURRENT` verificações de site em
  voo (10 por padrão), além de `DUMBMONIT_MAX_CONCURRENT_PROBES`.
- **Por domínio**: no máximo `DUMBMONIT_WEBSITE_MAX_PER_HOST` verificações em voo
  para um domínio (1 por padrão), e no mínimo
  `DUMBMONIT_WEBSITE_HOST_SPACING_SECS` segundos entre duas delas (2 por
  padrão, ou a **Aguardar antes da próxima solicitação para este domínio** do site).
  O domínio é o nome registrável: `www.example.com`, `shop.example.com`
  e `example.com` compartilham um, `example.co.uk` é um; subdomínios do cliente de
  plataformas de hospedagem (`*.myshopify.com`, `*.wordpress.com`, `*.github.io`,
  `*.netlify.app`…) cada um conta como seus próprios.

Um site cuja verificação anterior ainda aguarda sua vez pula uma rodada em vez
de enfileirar duas vezes. Muitos sites em um domínio com um intervalo de polling curto
são, portanto, verificados com menos frequência que solicitado: aumente o intervalo ou os
limites por domínio, se a lista de monitores os mostrar atrasados. Consulte
[Configuração](../reference/configuration.md#server).

## Detecção de plataforma {#platform-detection}

A plataforma vem de, em ordem: a tag `<meta name="generator">`, cabeçalhos
específicos de um host ou mecanismo (`x-shopify-*`, `x-wix-*`, `x-drupal-cache`,
`x-powered-by: Next.js`…), traços na marcação (`/wp-content/`,
`cdn.shopify.com`, `data-wf-site`, `__NEXT_DATA__`, `/_nuxt/`…) e finalmente o
host em si (`*.myshopify.com`, `*.wixsite.com`…). Reconhecido: WordPress,
Shopify, Wix, Squarespace, Webflow, Ghost, Drupal, Joomla, PrestaShop, Magento,
Next.js, Nuxt, Gatsby, Hugo, Jekyll; qualquer outra coisa é **Outro**. A plataforma
é o rótulo `platform` de `dumbmonit_website_info`.

## Importar uma lista de sites (CSV) {#import-a-list-of-sites-csv}

A partir da lista de monitores ou da página adicionar monitor, **Importar sites (CSV)** pega um
arquivo ou texto colado com uma linha de cabeçalho e até 1000 linhas:

| Coluna | Obrigatória | Significado |
|---|---|---|
| `url` | sim | Endereço do site. |
| `name` | não | Nome do monitor; o host por padrão. |
| `folder` | não | Pasta do monitor; vazio: `Websites/<plataforma>`, detectado lendo o site uma vez. |
| `tags` | não | `key=value;key=value`. |
| `interval` | não | Intervalo de polling. |
| `expect_text` | não | Igual à opção. |

```csv
url,name,folder,tags,interval,expect_text
https://example.com,Example,,client=acme,60s,Example Inc.
https://shop.example.org,Shop,Customers/Acme,,300s,Add to cart
```

Uma visualização mostra cada linha e seus erros antes de qualquer coisa ser criada; um site
já monitorado (mesmo endereço final) é ignorado.

## Métricas {#metrics}

| Métrica | Significado |
|---|---|
| `dumbmonit_website_up` | 1 quando a resposta final é um sucesso (2xx ou 3xx), 0 caso contrário. |
| `dumbmonit_website_status_code` | Status HTTP da resposta final. |
| `dumbmonit_website_response_seconds` | Tempo total, redirecionamentos incluídos. |
| `dumbmonit_website_redirect_hops` | Redirecionamentos seguidos. |
| `dumbmonit_website_https` | 1 quando o endereço final é https. |
| `dumbmonit_website_security_header{header}` | 0/1 por rótulo `header` (somente https): HSTS, nosniff, CSP, X-Frame-Options, frame-ancestors. |
| `dumbmonit_website_info{platform}` | Sempre 1; o rótulo leva a plataforma. |
| `dumbmonit_website_text_chars` | Caracteres de texto visível (respostas 2xx). |
| `dumbmonit_website_blank` | 1 quando esse texto está abaixo do limite. |
| `dumbmonit_website_expect_text_missing` | 1 quando o texto esperado está faltando (opção definida). |
| `dumbmonit_website_noindex` / `dumbmonit_website_nofollow` | Diretivas de indexação. |
| `dumbmonit_website_h1_count` / `dumbmonit_website_hreflang_count` | Títulos `<h1>`, `hreflang` alternados. |
| `dumbmonit_website_title_changed`, `…_description_changed`, `…_canonical_changed`, `…_final_url_changed` | 1 por 24 horas após uma mudança confirmada. |
| `dumbmonit_website_robots_blocks_all` | 1 quando `robots.txt` proíbe tudo para Googlebot. |
| `dumbmonit_website_sitemap_ok` | 1 quando um sitemap responde e lê. |

## Alertas integrados {#built-in-alerts}

| Alerta | Quando | Severidade |
|---|---|---|
| Monitor inacessível | Sem resposta, certificado recusado, loop de redirecionamento ou muitos redirecionamentos | crítico |
| Site inativo | A resposta final é um erro 4xx ou 5xx, por 3 minutos | crítico |
| Página do site em branco | Texto visível abaixo do limite, por 3 minutos | crítico |
| Texto esperado do site ausente | O texto esperado está faltando, por 3 minutos | aviso |
| Site não indexável | `noindex` na página, por 10 minutos | crítico |
| Site bloqueado por robots.txt | `robots.txt` proíbe tudo, por 10 minutos | crítico |
| URL canônica do site alterada | Mudança confirmada | aviso |
| Endereço do site alterado | O host ou caminho final mudou, confirmado | aviso |
| Título ou descrição do site alterado | Mudança confirmada | informação |
| Site lento | Mais de 5 segundos, por 10 minutos | aviso |

A verificação é executada no próprio servidor DumbMonit: não pode passar por um agente de retransmissão.
