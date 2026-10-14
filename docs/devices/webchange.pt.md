# Alterações de site {#website-changes}

Monitorize o *conteúdo* de uma página web, ou de um site inteiro, e seja
avisado quando mudar: uma página de preços ou termos, um site de
documentação, um aviso de manutenção de um fornecedor, uma página de
anúncio público, ou o seu próprio site depois de uma implantação. Cada
alteração é guardada com o texto antes e depois, um diff de linhas e,
quando um navegador está disponível, uma captura de ecrã de cada versão.

O que é comparado é o **texto visível** de cada página, não o seu HTML:
scripts, estilos, comentários, blocos `noscript`, `svg` e `template` são
descartados, os espaços em branco são colapsados e cada bloco (parágrafo,
título, item de lista, linha de tabela) torna-se uma linha. Um novo build
da mesma página, um atributo reordenado ou um novo nome de recurso para
invalidar a cache não altera nada; uma frase reformulada altera.

### Que parte da página {#what-part-of-the-page}

A **navegação, o cabeçalho, o rodapé e as barras laterais** do site ficam de
fora: são iguais em todas as páginas, por isso uma nova entrada de menu
marcaria todas as páginas como alteradas numa só verificação. Seguem os
papéis que o HTML lhes dá: todo `nav`, um `header` ou `footer` que não esteja
dentro de um `article`, `aside`, `main`, `nav` ou `section` (o cabeçalho de um
artigo é mantido), um `aside` fora de `article` e `main`, e qualquer elemento
com o papel `navigation`, `banner`, `contentinfo` ou `search`. As suas
ligações continuam a ser seguidas para encontrar as páginas de um site.
**Compare menus, header and footer** desativa isto.

**Watched area** restringe a comparação a alguns elementos, por exemplo
`main` ou `#content`; uma página onde nenhum é encontrado é lida inteira em
vez de vazia. **Excluded areas** deixa de fora mais elementos: um banner de
cookies, uma caixa de «últimas publicações», um espaço publicitário. Ambos
aceitam seletores simples, separados por vírgulas: uma etiqueta (`main`), um
id (`#content`), classes (`.cookie-banner`), atributos (`[role=alert]`,
`[data-ad]`), ou vários em conjunto (`div.notice[role=alert]`).
Descendentes (`main p`) e outros combinadores não são suportados.

## Página ou site {#page-or-site}

**Page** (a predefinição) lê o endereço do dispositivo e nada mais. Pode
também ser um documento de texto simples ou JSON, comparado linha a
linha.

**Site** começa a partir do endereço e segue toda página sob a mesma
pasta (`https://example.com/docs/intro.html` cobre tudo sob `/docs/`;
**Path prefix** escolhe outra pasta). As páginas são encontradas:

1. no sitemap do site: as que `robots.txt` anuncia (linhas `Sitemap:`),
   ou `/sitemap.xml`, índices de sitemap incluídos (um nível);
2. falhando isso, ou quando o sitemap não lista nada sob a pasta,
   seguindo as ligações de cada página, em largura primeiro.

Só páginas da mesma origem (esquema, anfitrião e porta) são seguidas. Quando
a página inicial redireciona para outro anfitrião do mesmo site
(`example.com` para `www.example.com`, `http` para `https`, ou um subdomínio
como `docs.example.com`), considera-se que o site vive aí, e as ligações ao
primeiro anfitrião são lidas no segundo. Quando redireciona para um site
completamente diferente (uma página de início de sessão como
`login.microsoftonline.com`, um domínio de terceiros), esse site não é
percorrido: só a página inicial é lida, e nenhuma página é reportada como
removida. Ligações a imagens, folhas de estilo, scripts, PDFs, arquivos e
outros ficheiros são ignoradas pela sua extensão, e qualquer resposta que
não seja HTML é ignorada. Os fragmentos (`#section`), os parâmetros de
rastreio (`utm_…`, `fbclid`, `gclid` e semelhantes) e as consultas vazias são
removidos, para que uma página seja um endereço; **Ignore query strings**
descarta toda a parte `?…`. O
`robots.txt` é respeitado para as páginas encontradas desta forma: o
grupo que nomeia `DumbMonit`, ou senão o grupo `*`, com `Allow`,
`Disallow`, `*` e `$`.

O rastreio é educado: no máximo dois pedidos de cada vez, meio segundo
entre dois lotes, o `User-Agent` `DumbMonit/<versão>`, um tempo limite de
15 segundos por pedido e no máximo 5 MB lidos por página. Para em **Max
pages** (25 por predefinição, 200 no máximo) ou depois de dez minutos. Quando
**Max pages** o parou, a página do dispositivo diz quantas páginas foram
encontradas, para que a parte não monitorizada do site nunca seja uma
surpresa. **User agent** substitui `DumbMonit/<versão>` para um site que
afasta robôs.

## Como funciona uma verificação {#how-a-check-works}

O dispositivo é sondado como qualquer outro (a cada minuto por
predefinição), mas as páginas só são lidas a cada **Check every** minutos
(60 por predefinição, de 5 a 10080). Entretanto, a sondagem só reporta o
estado da última verificação: o site não é atingido a cada minuto. Uma
verificação corre em segundo plano; duas verificações do mesmo
dispositivo nunca se sobrepõem, e **Check now** na página do dispositivo
inicia uma de imediato.

A primeira verificação só regista a **referência**: toda página é lida e
guardada, nada é reportado. A partir daí, para cada página:

| Situação | O que é guardado |
|---|---|
| Mesmo texto que da última vez (depois de **Ignore lines**) | Nada de novo; a página é marcada como verificada. |
| Texto diferente | Um novo instantâneo e um evento **changed**, com o número de linhas adicionadas e removidas. |
| Uma página nunca vista antes, ou vista de novo depois de ter desaparecido | Um instantâneo e um evento **new page**. |
| A página responde 404 ou 410 | Um evento **removed page**, o último instantâneo como «antes». |
| Âmbito de site: um rastreio completo já não alcança uma página conhecida | Um evento **removed page** também («No longer linked»). Um rastreio parado por **Max pages** ou pelo seu limite de tempo nunca conclui que uma página desapareceu. |
| Qualquer outro erro numa página (tempo limite, 403, 500 numa sub-página) | O erro é mostrado na página; nenhum evento. |

Alterar o endereço, o âmbito, o prefixo de caminho, **Max pages**, as áreas
ou **Ignore lines** torna a verificação seguinte uma nova referência em vez
de reportar toda página como alterada (ou, para um **Max pages** mais alto,
toda página que acrescenta como nova). Uma atualização do DumbMonit que lê as
páginas de forma diferente faz o mesmo uma vez.

Quando uma verificação encontra a **mesma alteração em várias páginas** (as
mesmas linhas removidas e acrescentadas, como um banner), a página do
dispositivo mostra-a uma só vez, com as páginas em que foi feita.

### Linhas a ignorar {#ignore-lines}

Linhas que mudam a cada visita — um relógio, um contador de visitantes,
«atualizado há 5 minutos», uma citação aleatória — seriam reportadas em
cada verificação. **Ignore lines** contém expressões regulares (sintaxe
`regex` do Rust), uma por linha ou unidas com `|`; qualquer linha de
texto que uma delas corresponda é deixada de fora antes de comparar. Por
exemplo:

```
Updated \d+ minutes ago|Visitors: \d+|© \d{4}
```

### Página inicial inacessível {#unreachable-start-page}

Quando a própria página inicial não pode ser lida (sem resposta, ligação
recusada, tempo limite, HTTP 5xx), o dispositivo está **inacessível**: a
sondagem falha, e a regra «Device unreachable» dispara como para
qualquer dispositivo. A página inicial é então tentada de novo em cada
sondagem, por isso o seu retorno é visto dentro de um minuto. Uma página
inicial que recusa o acesso (401, 403) é reportada como um problema de
configuração, não como uma indisponibilidade. Erros noutras páginas do
site só são mostrados nessas páginas.

Endereços que só o próprio anfitrião do DumbMonit pode alcançar
(loopback, link-local, o serviço de metadados da cloud) são recusados a
menos que **Allow loopback and link-local targets** esteja ativo;
endereços de LAN privada são sempre permitidos.

## Alertas {#alerts}

A regra incorporada **Website changed** (`webchange_detected`, Info)
dispara depois de uma verificação que encontrou pelo menos uma alteração
(página alterada, nova ou removida), e a notificação nomeia as páginas:
«/pricing, /contact e mais 2». Enquanto está a disparar, cada verificação
seguinte que encontra alterações é notificada de novo, com as suas próprias
páginas, mesmo nas mesmas páginas; o alerta resolve-se na primeira
verificação que não encontra nada de novo. Continua a ser um só alerta por
dispositivo, por isso reconhecê-lo ou suspendê-lo a partir do alerta cobre
também as verificações seguintes. Lê
`dumbmonit_webchange_last_check_changes`; associe-a a um canal como
qualquer outra regra, ou suba a sua severidade, em **Alerts → Rules**.
Ver [Regras incorporadas](../alerting/rules.md#website-changes).

| Métrica | O quê |
|---|---|
| `dumbmonit_webchange_pages` | Páginas seguidas (páginas removidas excluídas). |
| `dumbmonit_webchange_pages_failed` | Páginas seguidas cuja última leitura falhou. |
| `dumbmonit_webchange_last_check_changes` | Alterações encontradas pela última verificação. |

As métricas aparecem depois de a primeira verificação estar concluída.

## Capturas de ecrã {#screenshots}

A imagem DumbMonit não tem navegador: as capturas de ecrã vêm de um
Chromium headless opcional, controlado via Chrome DevTools Protocol.
Acrescente um como serviço `browser` ao lado do DumbMonit no
`docker-compose.yml` e aponte o servidor para ele:

```yaml
services:
  dumbmonit:
    environment:
      DUMBMONIT_BROWSER_URL: http://browser:9222
  browser:
    image: chromedp/headless-shell:stable
    shm_size: 256m
    restart: unless-stopped
```

Depois execute `docker compose up -d`. Nada publica a porta do navegador: só o
servidor DumbMonit fala com ele, pela rede do Compose.

`DUMBMONIT_BROWSER_URL` aponta o DumbMonit para o navegador (qualquer
Chromium iniciado com `--remote-debugging-port`, por exemplo
`chromedp/headless-shell`; o servidor lê `/json/version` e fala com o
endereço para o qual o nome de anfitrião resolve).

Uma captura de ecrã só é feita quando uma página recebe um novo
instantâneo (a referência, e cada alteração), nunca numa verificação sem
alterações: uma janela de 1366 × 900, a página completa até 5000 pixels,
como JPEG. A opção **Screenshots** desliga-as para um dispositivo. Sem um
navegador, ou quando não responde, a comparação de texto funciona da
mesma forma e os instantâneos simplesmente não têm imagem; a página do
dispositivo diz isso.

O navegador carrega a página como um visitante o faria, com os seus
scripts: o que mostra pode diferir do texto que o DumbMonit comparou, que
é o HTML que o servidor enviou.

## Armazenamento {#storage}

O texto de cada instantâneo é guardado na base de dados (1 MB no máximo
por página); as capturas de ecrã são ficheiros sob
`/data/webchange/<id do dispositivo>/`. Por dispositivo, as últimas 500
alterações são guardadas; um instantâneo é guardado enquanto for o mais
recente de uma página ou uma alteração guardada se referir a ele, e tudo
o resto, ficheiros incluídos, é removido depois de cada verificação.
Eliminar o dispositivo remove tudo.

## Monitorizar um site para alterações {#watch-a-website-for-changes}

Os passos abaixo são os que o aviso junto ao formulário mostra.

1. No endereço, cole a página a monitorizar, com http:// ou https://. Para monitorizar um site inteiro ou uma das suas secções, cole a sua página inicial e defina «Scope» para «site»: toda página sob a mesma pasta é seguida, até «Max pages».
2. Linhas que mudam a cada visita (um relógio, um contador de visitantes, «atualizado há 5 minutos») seriam reportadas cada vez: liste-as em «Ignore lines» como expressões regulares.
3. A primeira verificação só regista a referência, nada é reportado. A partir daí, cada verificação compara o texto visível página por página e guarda uma cópia antes/depois de cada alteração, mostrada na página do dispositivo; o alerta «Website changed» avisa-o.
4. Para uma captura de ecrã de cada versão, acrescente um navegador headless ao seu ficheiro Compose; as linhas exatas estão na documentação (Advanced options, «Screenshots of website changes»). Ver [Screenshots of website changes](../install/docker.md#website-screenshots).

!!! warning
    Só o texto que o servidor envia é comparado: uma página desenhada inteiramente por JavaScript mostra pouco ou nada à verificação. Páginas atrás de um início de sessão também não são alcançáveis.

Credenciais: nenhuma. As verificações correm sempre no servidor
DumbMonit: uma monitorização de site não pode passar por um agente relay.

## Opções {#options}

| Opção | Predefinição | Significado |
|---|---|---|
| Scope (`scope`) | `page` | `page`: só o endereço. `site`: toda página sob a mesma pasta. |
| Max pages (`max_pages`) | `25` | Âmbito de site: páginas lidas no máximo por verificação, 1 a 200. |
| Path prefix (`path_prefix`) | pasta do endereço | Âmbito de site: só páginas cujo caminho começa com isto (`/docs/`). |
| Ignore lines (`ignore`) | — | Expressões regulares; linhas de texto correspondentes não são comparadas. |
| Watched area (`content_selector`) | — | Seletores simples; só o texto dentro deles é comparado. |
| Excluded areas (`exclude_selector`) | — | Seletores simples; o seu texto não é comparado. |
| Compare menus, header and footer (`keep_page_chrome`) | `false` | Comparar também a navegação, o cabeçalho, o rodapé e as barras laterais do site. |
| Ignore query strings (`ignore_query`) | `false` | Âmbito de site: endereços que diferem só pela parte `?…` são uma só página. |
| User agent (`user_agent`) | — | Enviado em vez de `DumbMonit/<versão>`. |
| Screenshots (`screenshots`) | `true` | Guardar uma captura de ecrã de cada nova versão, quando um navegador está configurado. |
| Check every (`check_interval_minutes`) | `60` | Minutos entre duas verificações, 5 a 10080. |
| Accept an unverifiable certificate (`insecure_tls`) | `false` | Para um site próprio atrás de um certificado autoassinado. |
| Allow loopback and link-local targets (`allow_private_targets`) | `false` | Alcançar um site servido pelo próprio anfitrião do DumbMonit. |
