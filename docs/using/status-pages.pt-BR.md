# Páginas de status {#status-pages}

![A página Páginas de status: suas páginas, e os anúncios abaixo delas](../assets/screenshots/status-light.png){ loading=lazy }

Uma página de status é a face pública do seu monitoramento: uma página
que qualquer um pode abrir — sem login, sem cookie — que diz se seus
serviços estão ativos, mostra até 90 dias de histórico diário para cada
um, carrega seus anúncios de incidente e manutenção, e deixa visitantes
acompanhá-los por RSS ou e-mail. Badges colocam a mesma resposta em um
README; uma visão compacta cabe em uma página de intranet.

Ela só mostra o que você coloca nela: o rótulo que você dá a cada
serviço, seu estado, sua disponibilidade e seu tempo de resposta. Nomes
de dispositivo, endereços, tipos e identificadores nunca saem do
servidor — não na página, não em um badge, não em um e-mail.

## 1. Criar uma página {#1-create-a-page}

**Configurações → Páginas de status** → **Nova página**. Essa página lista suas
páginas de status (abrir, editar, excluir) com os anúncios abaixo
delas; o editor abre em sua própria rota (`/settings/status/new`,
`/settings/status/<id>`). A renderização pública está em `/s/<address>`.

| Campo | O que faz |
| --- | --- |
| **Título** | O título da página ("Home lab"). |
| **Endereço** | A última parte da URL, `/s/<address>`. Sugerido a partir do título; letras minúsculas, dígitos e hífens, 2 a 40 caracteres. |
| **Descrição** | Uma frase abaixo do título. Opcional. |
| **Tema** | Dia, noite, ou seguir o sistema do visitante. |
| **Histórico** | A janela mais longa que a barra de histórico cobre (30, 60 ou 90 dias). A barra se adapta à idade dos dados da página: mostra tantos dias quanto o serviço mais antigo foi monitorado, de 7 dias até esta configuração. Uma página nunca diz mais que isso — veja [Badges](#badges). |
| **Modo simples** | Uma página simples: sem cenário, sem mascote, sem animação e sem easter eggs. Claro ou escuro segue o sistema do visitante, com um alternador no cabeçalho; a configuração de Tema da página é ignorada. |
| **Publicada** | Desligado = rascunho: a página, seus badges e seu logo respondem "não encontrado" aos visitantes até você ativá-la. |

Depois marque os dispositivos a mostrar. Para cada um, defina o
**rótulo** que visitantes vão ler (o padrão é o nome do dispositivo —
você pode preferir "Site" a "nginx-front-01") e, opcionalmente, um
**grupo** ("Rede", "Armazenamento"). Grupos se tornam blocos na página,
na ordem em que você organiza os serviços.

Salve, e o link está pronto para copiar. A página se atualiza
automaticamente a cada minuto.

### Aparência {#look}

A seção **Aparência** do editor veste a página com suas cores sem abri-la
a código arbitrário:

| Configuração | O que faz |
| --- | --- |
| **Logo** | PNG, JPEG ou WebP, até 256 KiB, mostrado ao lado do título. Quadrado funciona melhor. SVG é recusado: pode carregar script, e a página é servida do mesmo endereço que a administração. O arquivo é verificado pelo seu conteúdo, armazenado sob `/data/status-pages/`, e servido apenas como imagem. |
| **Destaque** | Tinta (padrão), azul, verde-água, violeta, rosa ou âmbar — a régua superior, os links e o botão de inscrição. Cada um é escolhido para permanecer legível de dia e de noite. |
| **Site da organização** | Um link `http://` ou `https://` mostrado ao lado do título, como o nome de host do site. |
| **Texto de rodapé** | Texto simples abaixo da página, até 280 caracteres: quem a administra, como contatá-los. Quebras de linha são mantidas; nada é interpretado como HTML ou Markdown. |

### Cenário {#scene}

A configuração **Cenário** desenha um horizonte de cidade detrás do
banner. Páginas novas começam com Paris, e páginas existentes que não
tinham cenário agora também o mostram; desmarque-a para um banner
simples, ou ative o **Modo simples** para abandonar cenários por
completo. Escolha quantos quiser, na ordem que quiser. Cidades: Veneza,
Paris, Tóquio, Nova York, Londres, Roma, Sydney, Dubai, San Francisco,
Barcelona, Amsterdã, Istambul e Rio de Janeiro. As novas sete
maravilhas: Chichén Itzá, Machu Picchu, a Grande Muralha, Petra e o Taj
Mahal (o Cristo Redentor do Rio está no cenário do Rio). As maravilhas
antigas: Gizé, Babilônia, Éfeso, Olímpia, Halicarnasso, Rodes e
Alexandria, mais a Acrópole de Atenas. Com mais de um, **Rotação** decide
quando a página passa para o próximo: a cada visita (padrão), a cada
minuto, a cada dez minutos ou a cada hora. Os cenários são um conjunto
fechado de desenhos entregues com o DumbMonit — nada é enviado ou
interpretado.

Deliberadamente não há CSS ou HTML personalizado: uma página de status
é pública e vive na mesma origem que sua interface administrativa, sob a
mesma [política de segurança de conteúdo](../reference/api.md).

## 2. O que os visitantes veem {#2-what-visitors-see}

- Um banner com a resposta de um segundo, lida dos serviços: **Todos os
  sistemas operacionais**, **Indisponibilidade parcial** (alguns fora do
  ar ou degradados), **Indisponibilidade importante** (todos fora do
  ar) ou **Manutenção programada**. Um incidente aberto enquanto todo
  serviço ainda responde lê **Incidente em andamento**, tonalizado por
  seu impacto.
- Os anúncios abertos, atualização mais recente primeiro, com a linha
  do tempo de atualizações sob uma aba.
- Cada serviço com sua placa de estado (Operacional, Degradado, Fora do
  ar, Manutenção, Sem dados), sua **barra de histórico** — um quadrado
  por dia — sua disponibilidade em 24 h, 7 dias, 30 dias e (em uma
  página de 90 dias) 90 dias, e o tempo de resposta para sondagens de
  disponibilidade.
- **Incidentes passados**, por dia, para os últimos 30 dias.
- **Receber atualizações**: o feed RSS, e um formulário de inscrição por
  e-mail quando você configurar um (veja
  [Inscritos por e-mail](#email-subscribers)).

### A barra de histórico {#the-history-bar}

Cada quadrado é um dia (UTC), tonalizado pela disponibilidade daquele
dia: cor plena a 99,5% e acima, âmbar a partir de 95%, vermelho abaixo.
**Um dia sem nenhuma medição é cinza**, não verde — um serviço
adicionado na semana passada tem 83 dias cinza, não 83 perfeitos.
Passe o mouse sobre um quadrado, ou foque a barra e mova com as setas
(Home e End vão para as pontas), para ler a data, a disponibilidade, os
**minutos de indisponibilidade** e os incidentes que afetaram aquele
dia. As palavras estão lá também para leitores de tela: o status nunca
é contado apenas por cor.

Como o estado é decidido:

| Estado | Significado |
| --- | --- |
| Operacional | A última sondagem teve sucesso (sondagens de disponibilidade), ou o dispositivo reportou dentro de três períodos de sondagem. |
| Degradado | Ativo agora, mas pelo menos uma sondagem falhou na última hora. |
| Fora do ar | A última sondagem falhou, o dispositivo parou de reportar, ou sua configuração está com erro. |
| Manutenção | Uma janela de manutenção está em andamento e o serviço está fora do ar: esperado, não uma indisponibilidade. Ou um anúncio nesta página, ou uma [janela de manutenção](../alerting/maintenance.md) de alerta cobrindo esse dispositivo. |
| Sem dados | Nunca sondado ainda, ou desativado. |

Disponibilidade para sondagens (`http`, `tcp`, `dns`, `ping`, `tls`,
heartbeats) é a parcela de verificações bem sucedidas, de
`dumbmonit_probe_success`; a indisponibilidade de um dia é suas
verificações falhadas vezes o intervalo de sondagem. Para outros
dispositivos (SNMP, agent, Proxmox…) é a parcela de intervalos de cinco
minutos em que o dispositivo reportou ao menos uma vez
(`dumbmonit_up`), contado a partir da primeira medição — um dispositivo
adicionado ontem não está "fora do ar" pelos 89 dias anteriores a isso
— e a indisponibilidade de um dia são seus intervalos silenciosos.

## 3. Anunciar incidentes e manutenção {#3-announce-incidents-and-maintenance}

Em **Configurações → Páginas de status**, sob a lista de páginas, **Incidentes e
manutenção** → **Novo anúncio** (também ++ctrl+k++ → *Anunciar um
incidente*).

- Um **incidente** tem um título, um impacto (**menor** mostra a página
  como degradada, **maior** como uma indisponibilidade) e passa por
  *Investigando → Identificado → Monitorando → Resolvido*. Poste
  atualizações conforme avança: cada uma carrega um status e uma
  mensagem, e se torna a linha mais recente que visitantes leem.
  **Resolver** o encerra com uma mensagem final; ele então se move
  para "Incidentes passados" por 30 dias.
- Uma janela de **manutenção** tem um início e um fim e passa por
  *Programada → Em andamento → Concluída*. Enquanto está em andamento o
  banner diz "Manutenção programada" e serviços que estão fora do ar
  aparecem como *Manutenção*.

Uma [janela de manutenção programada em Alertas](../alerting/maintenance.md)
faz a mesma coisa para o dispositivo único que cobre, sem um anúncio:
esse serviço lê *Manutenção* em vez de vermelho enquanto a janela está
aberta, e o estado geral da página diz *Manutenção* quando nada mais
está fora do ar ou degradado. Só janelas que nomeiam um dispositivo
aparecem desse jeito; o nome e comentário da janela permanecem
privados.

Um anúncio é mostrado em uma página ou em **todas as páginas**.

### Inscritos por e-mail {#email-subscribers}

Visitantes podem se inscrever em seus anúncios por e-mail uma vez que
você escolha um **canal de e-mail (SMTP)** no editor, sob **Inscritos
por e-mail**. O canal é um que você já configurou sob
[Notificações](../alerting/notifications.md); seu servidor e remetente
são reutilizados, seus próprios destinatários não são. Sem um, a
página oferece apenas seu feed RSS.

- **Double opt-in.** Um visitante informa um endereço; o DumbMonit
  envia um link de confirmação, e nada mais é enviado até ele ser
  seguido. Um pedido nunca confirmado é esquecido depois de dois dias.
  O formulário responde a mesma frase se o endereço é novo, pendente ou
  já inscrito, então não diz a ninguém quem segue sua página.
- **O que é enviado.** Um e-mail quando um anúncio é criado, quando
  uma atualização é postada, e quando seu status muda — tanto para
  incidentes quanto manutenção, naquela página (ou em todas, para um
  anúncio mostrado em todas).
- **Cancelando a inscrição.** Todo e-mail termina com um link para
  cancelar a inscrição, e carrega os cabeçalhos `List-Unsubscribe` e
  `List-Unsubscribe-Post`, então Gmail, Apple Mail e outros oferecem seu
  próprio botão de um clique.
- **Limites.** O endpoint público de inscrição aceita cinco
  requisições por endereço de cliente e sessenta no total por 15
  minutos, e um endereço pendente não recebe e-mail duas vezes dentro
  de dez minutos: a página não pode ser transformada em um canhão de
  e-mail.
- O editor lista inscritos (confirmados ou pendentes) e remove
  qualquer um deles.

Os links nesses e-mails apontam para o
[domínio personalizado](#custom-domain) da página quando tem um
(`https://status.example.com/`). Caso contrário são construídos a
partir da **URL pública** da
[política de notificação](../alerting/notifications.md) (ou
`DUMBMONIT_PUBLIC_URL`); sem uma, do endereço que você usou quando
salvou a página por último — nunca da requisição de um visitante.

## 4. Compartilhar o link {#4-share-the-link}

A página vive em `https://your-server/s/<address>`. Tudo sob `/s/` e
`/api/public/` é servido sem autenticação; o resto do DumbMonit
permanece detrás do login.

O painel **Compartilhar** do editor constrói os trechos abaixo para
você, com uma pré-visualização.

### Badges {#badges}

Toda página publicada serve badges SVG — planos, com texto branco em
cor sólida, legíveis tanto no tema claro quanto escuro de um README do
GitHub:

| Badge | URL |
| --- | --- |
| Status da página | `/api/public/status/<address>/badge.svg` |
| Disponibilidade da página | `/api/public/status/<address>/uptime.svg?days=30` |
| Tempo de resposta da página | `/api/public/status/<address>/response.svg` |
| Status do serviço | `/api/public/status/<address>/components/<service>/badge.svg` |
| Disponibilidade do serviço | `/api/public/status/<address>/components/<service>/uptime.svg?days=30` |
| Tempo de resposta do serviço | `/api/public/status/<address>/components/<service>/response.svg` |

`<service>` é o rótulo do serviço na forma de URL: "API Pública" se
torna `api-publica` (uma segunda "API Pública" seria `api-publica-2`).
Também é a `key` de cada serviço no documento JSON. `days` é `1`, `7`,
`30` (padrão) ou `90`, e nunca mais que o histórico da página: uma
página de 30 dias recusa um badge de 90 dias. A disponibilidade e tempo
de resposta da página são a média sobre seus serviços. Um serviço sem
nada medido diz **sem dados**, em cinza.

Em um README:

```markdown
[![Status](https://your-server/api/public/status/home-lab/badge.svg)](https://your-server/s/home-lab)
[![Uptime](https://your-server/api/public/status/home-lab/uptime.svg?days=30)](https://your-server/s/home-lab)
[![API](https://your-server/api/public/status/home-lab/components/public-api/uptime.svg)](https://your-server/s/home-lab)
```

Badges são armazenados em cache por um minuto
(`Cache-Control: public, max-age=60`), então o proxy de imagem do
GitHub e seus visitantes não custam nada ao banco de dados de séries
temporais.

### Embutir {#embed}

`/s/<address>/embed` é uma visão compacta para um iframe: o estado
geral e uma linha por serviço com seu estado e 30 dias de histórico, e
um link para a página completa em uma nova aba.

```html
<iframe src="https://your-server/s/home-lab/embed"
        title="Home lab status" width="100%" height="320" style="border:0"></iframe>
```

Adicione `?theme=light` ou `?theme=dark` para corresponder à página
hospedeira, e `?history=0` para ocultar as barras. A página de status e
seu embed são os únicos endereços do DumbMonit que uma página de
terceiros pode colocar em um frame; tudo mais (as páginas de
confirmação e cancelamento de inscrição incluídas) responde com
`frame-ancestors 'none'` e `X-Frame-Options: DENY`.

### Domínio personalizado {#custom-domain}

Uma página pode ter seu próprio endereço, como
`status.example.com`, da forma que o Uptime Kuma faz. Digite o nome no
campo **Domínio público** do editor (o **i** ao lado resume os passos)
e salve. Quando uma requisição alcança o DumbMonit com esse nome em seu
cabeçalho `Host`, o DumbMonit mostra a página na raiz `/`, e nada mais:
nesse nome a tela de login, as configurações, a API, os endpoints de
agentes, as métricas e as outras páginas de status todas respondem
`404 Not found`, mesmo para alguém conectado, e nenhum cookie de sessão
é lido ou definido. Só a página, seu embed, suas páginas de confirmação
e cancelamento de inscrição, seus badges, feed e documento JSON, e os
próprios arquivos da interface (`/_app/`, ícones) são servidos.

O nome é armazenado em minúsculas, sem `https://` ou uma barra final;
uma porta, um caminho ou um endereço IP é recusado. Duas páginas não
podem compartilhar um nome, e o endereço que você usa para alcançar o
próprio DumbMonit (ou sua URL pública) não pode ser o domínio de uma
página — salvá-lo o trancaria fora. Limpe o campo para remover o
domínio; a página permanece em `/s/<address>` de qualquer forma.

Para configurá-lo:

1. **DNS.** Crie o registro para o nome (ou o nome de host público de
   um Cloudflare Tunnel), apontando para seu proxy reverso.
2. **Proxy.** Envie o nome para o endereço do DumbMonit, por exemplo
   `http://dumbmonit:8080`, **sem reescrever o caminho**: o proxy
   encaminha `/`, não `/s/<address>`.
3. **Cabeçalho Host.** Passe o cabeçalho `Host` original: é como o
   DumbMonit distingue o domínio da página de seu próprio endereço. O
   DumbMonit só lê `Host`; `X-Forwarded-Host` só conta de um proxy
   listado em `DUMBMONIT_TRUSTED_PROXIES`.
4. **HTTPS** é tratado pelo proxy. O DumbMonit liga para
   `https://<domain>/` em e-mails de inscritos, no feed RSS e no
   painel **Compartilhar** do editor.

Traefik (labels Docker) — o cabeçalho `Host` é passado por padrão:

```yaml
labels:
  - traefik.enable=true
  - traefik.http.routers.status.rule=Host(`status.example.com`)
  - traefik.http.routers.status.entrypoints=websecure
  - traefik.http.routers.status.tls.certresolver=letsencrypt
  - traefik.http.services.status.loadbalancer.server.port=8080
```

Nginx Proxy Manager — adicione um **Proxy Host** com o domínio
`status.example.com`, esquema `http`, hostname de encaminhamento
`dumbmonit`, porta `8080`, e um certificado SSL. Nada mais: ele passa
`Host` por padrão. Deixe **Custom locations** vazio.

Cloudflare Tunnel — adicione um **Public hostname**
`status.example.com` com o serviço `http://dumbmonit:8080` e nenhum
caminho. O tunnel mantém o `Host` do visitante; deixe **HTTP Host
Header** nas configurações avançadas vazio.

Caddy — `reverse_proxy` passa `Host` e o Caddy busca o certificado:

```caddyfile
status.example.com {
    reverse_proxy dumbmonit:8080
}
```

Nginx — `proxy_pass` envia o nome do upstream a menos que configurado
diferente, então defina o cabeçalho:

```nginx
server {
    listen 443 ssl;
    server_name status.example.com;
    # ssl_certificate … ;

    location / {
        proxy_pass http://dumbmonit:8080;
        proxy_set_header Host $host;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }
}
```

**Solução de problemas.**

- *O domínio mostra a tela de login ou painel do DumbMonit*: o
  cabeçalho `Host` não alcançou o DumbMonit — o proxy o substituiu
  pelo nome do upstream (`dumbmonit:8080`). Adicione
  `proxy_set_header Host $host;` (Nginx), remova um **HTTP Host
  Header** personalizado (Cloudflare Tunnel), ou remova um middleware
  de reescrita de `Host` (Traefik). Nada privado é exposto: o painel
  ainda pede uma senha.
- *Um `Not found` simples no domínio*: o proxy reescreve o caminho (ele
  envia `/s/<address>` ou adiciona um prefixo), ou o domínio no editor
  não corresponde exatamente ao nome no navegador. Encaminhe o caminho
  intacto.
- *A página diz que não existe*: a página ainda é um rascunho —
  publique-a.
- *Você não consegue mais alcançar a interface do DumbMonit*: você está
  navegando através do domínio da página. Use o próprio endereço do
  DumbMonit, ou limpe o **Domínio público** da página.

### Detrás de um proxy reverso {#behind-a-reverse-proxy}

Um [domínio personalizado](#custom-domain) é a forma mais simples de
expor só a página de status. Se você prefere manter o caminho
`/s/<address>` em um nome público, filtre os caminhos no proxy em vez
disso. Com Caddy, por exemplo:

```caddyfile
status.example.com {
    @public path /s/* /api/public/* /_app/* /favicon.svg
    reverse_proxy @public dumbmonit:8080
    respond 404
}
```

Com nginx:

```nginx
location ~ ^/(s/|api/public/|_app/|favicon\.svg) {
    proxy_pass http://dumbmonit:8080;
    proxy_set_header Host $host;
    proxy_set_header X-Forwarded-Proto $scheme;
}
location / { return 404; }
```

`/_app/` carrega os scripts e estilos da interface: a página precisa
deles para renderizar. Encaminhe `X-Forwarded-Proto` e
`X-Forwarded-Host` para que o feed RSS ligue para o endereço certo, e
defina a URL pública (veja acima) para que e-mails de inscritos também
façam.

### Feed e documento {#feed-and-document}

- `GET /api/public/status/<address>/rss` — um feed RSS de incidentes e
  janelas de manutenção, um item por anúncio com sua atualização mais
  recente.
- `GET /api/public/status/<address>` — o documento JSON a partir do
  qual a página é construída, se você quiser construir o seu próprio:
  o título, descrição e aparência da página, o estado geral, a `key`,
  `label`, `state`, disponibilidade (`uptime_24h`, `uptime_7d`,
  `uptime_30d`, `uptime_90d`), `latency_ms` e `history` (`date`,
  `uptime_pct`, `down_minutes`, `incidents` por dia) de cada serviço, e
  os anúncios.

Qualquer site pode ler esse documento do navegador (ele é enviado com
`Access-Control-Allow-Origin: *` e sem cookies).
`GET /api/public/status/<address>/banner.js` é um script pronto que o
transforma em um banner: veja
[Banner para o site](../integrations/website-banner.md).

O documento é armazenado em cache por 30 segundos no servidor; os
badges, o feed e o embed todos leem esse mesmo documento em cache,
então nenhum deles pode dizer mais que a página.
