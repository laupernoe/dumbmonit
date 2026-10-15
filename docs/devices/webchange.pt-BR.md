# Mudanças no site {#website-changes}

Observe o *conteúdo* de uma página web, ou de um site inteiro, e seja
avisado quando mudar: uma página de preços ou termos, um site de
documentação, um aviso de manutenção de um fornecedor, uma página de
anúncio público, ou seu próprio site depois de uma implantação. Cada
mudança é mantida com o texto antes e depois, um diff de linhas e, quando
um navegador está disponível, uma captura de tela de cada versão.

O que é comparado é o **texto visível** de cada página, não seu HTML:
scripts, estilos, comentários, blocos `noscript`, `svg` e `template` são
descartados, espaços em branco são colapsados e cada bloco (parágrafo,
título, item de lista, linha de tabela) se torna uma linha. Um novo build
da mesma página, um atributo reordenado ou um novo nome de arquivo de
cache-busting não mudam nada; uma frase reescrita muda.

### Que parte da página {#what-part-of-the-page}

A **navegação, o cabeçalho, o rodapé e as barras laterais** do site ficam
de fora: são iguais em todas as páginas, então uma nova entrada de menu
marcaria todas as páginas como mudadas em uma única verificação. Seguem os
papéis que o HTML lhes dá: todo `nav`, um `header` ou `footer` que não esteja
dentro de um `article`, `aside`, `main`, `nav` ou `section` (o cabeçalho de
um artigo é mantido), um `aside` fora de `article` e `main`, e qualquer
elemento com o papel `navigation`, `banner`, `contentinfo` ou `search`. Seus
links ainda são seguidos para encontrar as páginas de um site. **Comparar
menus, cabeçalho e rodapé** desativa isso.

**Área observada** restringe a comparação a alguns elementos, por exemplo
`main` ou `#content`; uma página em que nenhum deles é encontrado é lida
inteira em vez de vazia. **Áreas excluídas** deixa mais elementos de fora: um
banner de cookies, uma caixa de "últimas publicações", um espaço de anúncio.
Ambos aceitam seletores simples, separados por vírgulas: uma tag (`main`), um
id (`#content`), classes (`.cookie-banner`), atributos (`[role=alert]`,
`[data-ad]`), ou vários juntos (`div.notice[role=alert]`). Descendentes
(`main p`) e outros combinadores não são suportados.

## Página ou site {#page-or-site}

**Página** (o padrão) lê o endereço do monitor e nada mais. Também
pode ser um documento de texto simples ou JSON, comparado linha por
linha.

**Site** começa do endereço e segue toda página sob a mesma pasta
(`https://example.com/docs/intro.html` cobre tudo sob `/docs/`;
**Prefixo de caminho** escolhe outra pasta). Páginas são encontradas:

1. no sitemap do site: as que `robots.txt` anuncia (linhas `Sitemap:`),
   ou `/sitemap.xml`, índices de sitemap incluídos (um nível);
2. faltando isso, ou quando o sitemap não lista nada sob a pasta,
   seguindo os links de cada página, em largura.

Apenas páginas da mesma origem (esquema, host e porta) são seguidas. Quando
a página inicial redireciona para outro host do mesmo site (`example.com`
para `www.example.com`, `http` para `https`, ou um subdomínio como
`docs.example.com`), considera-se que o site mora lá, e os links para o
primeiro host são lidos no segundo. Quando redireciona para outro site
completamente diferente (uma página de login como
`login.microsoftonline.com`, um domínio de terceiros), esse site não é
varrido: só a página inicial é lida, e nenhuma página é reportada como
removida. Links para imagens, folhas de estilo, scripts, PDFs, arquivos
compactados e outros arquivos são pulados por sua extensão, e qualquer
resposta que não seja HTML é ignorada. Fragmentos (`#section`), parâmetros
de rastreamento (`utm_…`, `fbclid`, `gclid` e similares) e consultas vazias
são removidos, então uma página é um endereço; **Ignorar query strings**
descarta toda a parte `?…`.
`robots.txt` é respeitado para as páginas encontradas desse jeito: o
grupo que nomeia `DumbMonit`, ou senão o grupo `*`, com `Allow`,
`Disallow`, `*` e `$`.

A varredura é educada: duas requisições por vez no máximo, meio segundo
entre dois lotes, o `User-Agent` `DumbMonit/<version>`, um tempo limite
de 15 segundos por requisição e 5 MB lidos no máximo por página. Ela para
em **Máximo de páginas** (25 por padrão, 200 no máximo) ou depois de dez
minutos. Quando **Máximo de páginas** a interrompeu, a página do monitor
diz quantas páginas foram encontradas, para que a parte não observada do
site nunca seja uma surpresa. **User agent** substitui `DumbMonit/<version>`
para um site que barra robôs.

## Como uma verificação funciona {#how-a-check-works}

O monitor é sondado como qualquer outro (a cada minuto por padrão),
mas as páginas só são lidas a cada **Verificar cada** minutos (60 por
padrão, 5 a 10080). No intervalo, a sondagem só reporta o estado da
última verificação: o site não é atingido a cada minuto. Uma verificação
roda em segundo plano; duas verificações do mesmo monitor nunca se
sobrepõem, e **Verificar agora** na página do monitor inicia uma
imediatamente.

A primeira verificação só registra a **referência**: toda página é lida
e mantida, nada é reportado. A partir de então, para cada página:

| Situação | O que é mantido |
|---|---|
| Mesmo texto que a última vez (depois de **Ignorar linhas**) | Nada novo; a página é marcada como verificada. |
| Texto diferente | Um novo snapshot e um evento de **mudança**, com o número de linhas adicionadas e removidas. |
| Uma página nunca vista antes, ou vista de novo depois de ter desaparecido | Um snapshot e um evento de **página nova**. |
| A página responde 404 ou 410 | Um evento de **página removida**, o último snapshot como "antes". |
| Escopo de site: uma varredura completa não alcança mais uma página conhecida | Um evento de **página removida** também ("Não mais vinculada"). Uma varredura interrompida por **Máximo de páginas** ou por seu limite de tempo nunca conclui que uma página desapareceu. |
| Qualquer outro erro em uma página (tempo esgotado, 403, 500 em uma subpágina) | O erro é mostrado na página; nenhum evento. |

Mudar o endereço, o escopo, o prefixo de caminho, **Máximo de páginas**, as
áreas ou **Ignorar linhas** faz a próxima verificação uma nova referência em
vez de reportar toda página como mudada (ou, para um **Máximo de páginas**
maior, toda página que ele acrescenta como nova). Uma atualização do
DumbMonit que lê as páginas de outro jeito faz o mesmo uma vez.

Quando uma verificação encontra a **mesma mudança em várias páginas** (as
mesmas linhas removidas e adicionadas, como um banner), a página do
monitor a mostra uma só vez, com as páginas em que foi feita.

### Ignorar linhas {#ignore-lines}

Linhas que mudam a cada visita — um relógio, um contador de visitantes,
"atualizado há 5 minutos", uma citação aleatória — seriam reportadas a
cada verificação. **Ignorar linhas** mantém expressões regulares
(sintaxe `regex` do Rust), uma por linha ou unidas com `|`; qualquer
linha de texto que uma delas corresponde é deixada de fora antes da
comparação. Por exemplo:

```
Updated \d+ minutes ago|Visitors: \d+|© \d{4}
```

### Página inicial inacessível {#unreachable-start-page}

Quando a própria página inicial não pode ser lida (sem resposta, conexão
recusada, tempo esgotado, HTTP 5xx), o monitor está **inacessível**:
a sondagem falha, e a regra "Monitor unreachable" dispara como para
qualquer monitor. A página inicial é então tentada de novo a cada
sondagem, então seu retorno é visto dentro de um minuto. Uma página
inicial que recusa acesso (401, 403) é reportada como um problema de
configuração, não uma indisponibilidade. Erros em outras páginas do site
só são mostrados nessas páginas.

Endereços que só o próprio host DumbMonit pode alcançar (loopback,
link-local, o serviço de metadados de nuvem) são recusados a menos que
**Permitir alvos de loopback e link-local** esteja ativado; endereços de
LAN privados são sempre permitidos.

## Alertas {#alerts}

A regra integrada **Website changed** (`webchange_detected`, Info)
dispara depois de uma verificação que encontrou ao menos uma mudança
(mudada, nova ou página removida), e a notificação nomeia as páginas:
"/pricing, /contact e mais 2". Enquanto está disparando, toda nova
verificação que encontra mudanças é notificada de novo, com suas próprias
páginas, mesmo nas mesmas páginas; o alerta resolve na primeira verificação
que não encontra nada novo. Continua sendo um alerta por monitor o
tempo todo, então reconhecê-lo ou adiá-lo pelo alerta cobre também as
verificações seguintes. Ela lê `dumbmonit_webchange_last_check_changes`;
conecte-a a um canal como qualquer outra regra, ou aumente sua
severidade, em **Alertas → Regras**. Veja
[Regras integradas](../alerting/rules.md#website-changes).

| Métrica | O que é |
|---|---|
| `dumbmonit_webchange_pages` | Páginas seguidas (páginas removidas excluídas). |
| `dumbmonit_webchange_pages_failed` | Páginas seguidas cuja última leitura falhou. |
| `dumbmonit_webchange_last_check_changes` | Mudanças encontradas pela última verificação. |

As métricas aparecem uma vez que a primeira verificação é feita.

## Capturas de tela {#screenshots}

A imagem DumbMonit não tem navegador: capturas de tela vêm de um Chromium
headless opcional, controlado via Chrome DevTools Protocol. Adicione um como serviço `browser` ao lado do DumbMonit em `docker-compose.yml`
e aponte o servidor para ele:

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
endereço para o qual o nome de host resolve).

Uma captura de tela é tirada apenas quando uma página recebe um novo
snapshot (a referência, e toda mudança), nunca em uma verificação sem
mudança: uma janela de 1366 × 900, a página completa até 5000 pixels,
como JPEG. A opção **Capturas de tela** as desliga para um monitor.
Sem um navegador, ou quando ele não responde, a comparação de texto
funciona da mesma forma e os snapshots simplesmente não têm imagem; a
página do monitor avisa isso.

O navegador carrega a página como um visitante faria, com seus scripts:
o que mostra pode diferir do texto que o DumbMonit comparou, que é o
HTML que o servidor enviou.

## Armazenamento {#storage}

O texto de cada snapshot é mantido no banco de dados (1 MB no máximo por
página); capturas de tela são arquivos sob
`/data/webchange/<device id>/`. Por monitor, as últimas 500 mudanças
são mantidas; um snapshot é mantido enquanto é o mais recente de uma
página ou uma mudança mantida se refere a ele, e tudo mais, arquivos
incluídos, é removido depois de cada verificação. Excluir o monitor
remove tudo isso.

## Observar um site para mudanças {#watch-a-website-for-changes}

Os passos abaixo são os que o aviso ao lado do formulário mostra.

1. No endereço, cole a página a observar, com http:// ou https://. Para
   observar um site inteiro ou uma de suas seções, cole sua página
   inicial e defina "Escopo" para "site": toda página sob a mesma pasta
   é seguida, até "Máximo de páginas".
2. Linhas que mudam a cada visita (um relógio, um contador de
   visitantes, "atualizado há 5 minutos") seriam reportadas a cada vez:
   liste-as em "Ignorar linhas" como expressões regulares.
3. A primeira verificação só registra a referência, nada é reportado. A
   partir de então, cada verificação compara o texto visível página por
   página e mantém uma cópia antes/depois de toda mudança, mostrada na
   página do monitor; o alerta "Website changed" avisa você.
4. Para uma captura de tela de cada versão, adicione um navegador headless ao
   seu arquivo Compose; as linhas exatas estão na documentação (Opções
   avançadas, "Capturas de tela das mudanças de sites").
   Veja [Capturas de tela das mudanças de sites](../install/docker.md#website-screenshots).

!!! warning
    Só o texto que o servidor envia é comparado: uma página desenhada
    inteiramente por JavaScript mostra pouco ou nada à verificação.
    Páginas detrás de um login também não são alcançáveis.

Credenciais: nenhuma. Verificações sempre rodam no servidor DumbMonit:
uma observação de site não pode passar por um agente relay.

## Opções {#options}

| Opção | Padrão | Significado |
|---|---|---|
| Escopo (`scope`) | `page` | `page`: apenas o endereço. `site`: toda página sob a mesma pasta. |
| Máximo de páginas (`max_pages`) | `25` | Escopo de site: páginas lidas no máximo por verificação, 1 a 200. |
| Prefixo de caminho (`path_prefix`) | pasta do endereço | Escopo de site: só páginas cujo caminho começa com isso (`/docs/`). |
| Ignorar linhas (`ignore`) | — | Expressões regulares; linhas de texto correspondentes não são comparadas. |
| Área observada (`content_selector`) | — | Seletores simples; só o texto dentro deles é comparado. |
| Áreas excluídas (`exclude_selector`) | — | Seletores simples; o texto deles não é comparado. |
| Comparar menus, cabeçalho e rodapé (`keep_page_chrome`) | `false` | Compara também a navegação, o cabeçalho, o rodapé e as barras laterais do site. |
| Ignorar query strings (`ignore_query`) | `false` | Escopo de site: endereços que diferem só pela parte `?…` são uma página. |
| User agent (`user_agent`) | — | Enviado no lugar de `DumbMonit/<version>`. |
| Capturas de tela (`screenshots`) | `true` | Mantém uma captura de tela de cada versão nova, quando um navegador está configurado. |
| Verificar cada (`check_interval_minutes`) | `60` | Minutos entre duas verificações, 5 a 10080. |
| Aceitar um certificado não verificável (`insecure_tls`) | `false` | Para um site próprio detrás de um certificado autoassinado. |
| Permitir alvos de loopback e link-local (`allow_private_targets`) | `false` | Alcançar um site servido pelo próprio host DumbMonit. |
