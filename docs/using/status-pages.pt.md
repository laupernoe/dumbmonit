# Páginas de estado {#status-pages}

![A página Status pages: as suas páginas, e os anúncios sob elas](../assets/screenshots/status-light.png){ loading=lazy }

Uma página de estado é a face pública da sua monitorização: uma página
que qualquer um pode abrir — sem iniciar sessão, sem cookie — que diz
se os seus serviços estão ativos, mostra até 90 dias de histórico
diário para cada um, transporta os seus anúncios de incidente e
manutenção, e deixa os visitantes segui-la por RSS ou email. Os
emblemas colocam a mesma resposta num README; uma vista compacta cabe
numa página de intranet.

Só mostra o que colocar nela: o rótulo que dá a cada serviço, o seu
estado, a sua disponibilidade e o seu tempo de resposta. Os nomes,
endereços, tipos e identificadores de monitor nunca saem do
servidor — nem na página, nem num emblema, nem num email.

## 1. Criar uma página {#1-create-a-page}

**Settings → Status pages** → **New page**. Essa página lista as suas
páginas de estado (abrir, editar, eliminar) com os anúncios sob elas;
o editor abre na sua própria rota (`/settings/status/new`, `/settings/status/<id>`). A
representação pública está em `/s/<address>`.

| Campo | O que faz |
| --- | --- |
| **Title** | O título da página («Home lab»). |
| **Address** | A última parte do URL, `/s/<address>`. Sugerida a partir do título; letras minúsculas, dígitos e hífenes, 2 a 40 caracteres. |
| **Description** | Uma frase sob o título. Opcional. |
| **Theme** | Dia, noite, ou seguir o sistema do visitante. |
| **History** | A janela mais longa que a barra de histórico cobre (30, 60 ou 90 dias). A barra adapta-se à idade dos dados da página: mostra tantos dias quanto o serviço mais antigo tem sido monitorizado, de 7 dias até esta definição. Uma página nunca diz mais do que isto — ver [Emblemas](#badges). |
| **Simple mode** | Uma página simples: sem cenário, sem mascote, sem animação e sem easter eggs. Claro ou escuro segue o sistema do visitante, com um alternador no cabeçalho; a definição Theme da página é ignorada. |
| **Published** | Desligado = rascunho: a página, os seus emblemas e o seu logo respondem «not found» aos visitantes até a ativar. |

Depois marque os monitores a mostrar. Para cada um, defina o
**rótulo** que os visitantes lêem (a predefinição é o nome do
monitor — pode preferir «Website» a «nginx-front-01») e,
opcionalmente, um **grupo** («Network», «Storage»). Os grupos tornam-se
blocos na página, pela ordem em que organiza os serviços.

Guarde, e a ligação está pronta para copiar. A página atualiza-se a
cada minuto.

### Look {#look}

A secção **Look** do editor veste a página nas suas cores sem a abrir
a código arbitrário:

| Definição | O que faz |
| --- | --- |
| **Logo** | PNG, JPEG ou WebP, até 256 KiB, mostrado junto ao título. Quadrado funciona melhor. SVG é recusado: pode transportar script, e a página é servida do mesmo endereço que a administração. O ficheiro é verificado pelo seu conteúdo, guardado sob `/data/status-pages/`, e servido só como imagem. |
| **Accent** | Ink (predefinição), blue, teal, violet, rose ou amber — a regra do topo, as ligações e o botão de subscrição. Cada uma é escolhida para ficar legível de dia e de noite. |
| **Organisation website** | Uma ligação `http://` ou `https://` mostrada junto ao título, como o nome de anfitrião do site. |
| **Footer text** | Texto simples sob a página, até 280 caracteres: quem a gere, como contactar. As quebras de linha são mantidas; nada é interpretado como HTML ou Markdown. |

### Scene {#scene}

A definição **Scene** desenha um horizonte de cidade atrás da faixa.
As páginas novas começam com Paris, e as páginas existentes que não
tinham cenário também o mostram agora; desmarque-o para uma faixa
simples, ou ative **Simple mode** para abandonar os cenários de todo.
Escolha quantos quiser, pela ordem que quiser. Cidades: Veneza, Paris,
Tóquio, Nova Iorque, Londres, Roma, Sidney, Dubai, São Francisco,
Barcelona, Amesterdão, Istambul e Rio de Janeiro. As sete novas
maravilhas: Chichén Itzá, Machu Picchu, a Grande Muralha, Petra e o Taj
Mahal (o Cristo Redentor do Rio está no cenário do Rio). As maravilhas
antigas: Gize, Babilónia, Éfeso, Olímpia, Halicarnasso, Rodes e
Alexandria, mais a Acrópole de Atenas. Com mais de um, **Rotation**
decide quando a página passa ao seguinte: a cada visita (predefinição),
a cada minuto, a cada dez minutos ou a cada hora. Os cenários são um
conjunto fechado de desenhos que vêm com o DumbMonit — nada é carregado
ou interpretado.

Não há, de propósito, CSS ou HTML personalizado: uma página de estado
é pública e vive na mesma origem que a sua interface de administração,
sob a mesma [política de segurança de conteúdo](../reference/api.md).

## 2. O que os visitantes veem {#2-what-visitors-see}

- Uma faixa com a resposta de um segundo, lida dos serviços: **All
  systems operational**, **Partial outage** (alguns em baixo ou
  degradados), **Major outage** (todos em baixo) ou **Scheduled
  maintenance**. Um incidente aberto enquanto todo serviço ainda
  responde lê-se **Incident in progress**, matizado pelo seu impacto.
- Os anúncios abertos, a atualização mais recente primeiro, com o
  histórico de atualizações sob uma dobra.
- Cada serviço com a sua placa de estado (Operational, Degraded, Down,
  Maintenance, No data), a sua **barra de histórico** — um quadrado
  por dia — a sua disponibilidade em 24 h, 7 dias, 30 dias e (numa
  página de 90 dias) 90 dias, e o tempo de resposta para sondas de
  disponibilidade.
- **Past incidents**, por dia, nos últimos 30 dias.
- **Get updates**: o feed RSS, e um formulário de subscrição por email
  quando configura um (ver [Subscritores por email](#email-subscribers)).

### A barra de histórico {#the-history-bar}

Cada quadrado é um dia (UTC), matizado pela disponibilidade desse dia:
cor completa a 99,5% e acima, âmbar a partir de 95%, vermelho abaixo.
**Um dia sem nenhuma medição é cinza**, não verde — um serviço
adicionado na semana passada tem 83 dias cinzentos, não 83 perfeitos.
Passe o cursor sobre um quadrado, ou foque a barra e mova-se com as
teclas de seta (Home e End saltam para as pontas), para ler a data, a
disponibilidade, os **minutos de indisponibilidade** e os incidentes
que afetaram esse dia. As palavras também estão lá para leitores de
ecrã: o estado nunca é dito só pela cor.

Como o estado é decidido:

| Estado | Significado |
| --- | --- |
| Operational | A última verificação teve sucesso (sondas de disponibilidade), ou o monitor reportou dentro de três períodos de sondagem. |
| Degraded | Ativo agora, mas pelo menos uma verificação falhou na última hora. |
| Down | A última verificação falhou, o monitor deixou de reportar, ou a sua configuração está em erro. |
| Maintenance | Uma janela de manutenção está em curso e o serviço está em baixo: esperado, não uma indisponibilidade. Ou um anúncio nesta página, ou uma [janela de manutenção](../alerting/maintenance.md) de alertagem que cobre esse monitor. |
| No data | Nunca verificado ainda, ou desativado. |

A disponibilidade para sondas (`http`, `tcp`, `dns`, `ping`, `tls`,
heartbeats) é a fração de verificações bem-sucedidas, a partir de
`dumbmonit_probe_success`; a indisponibilidade de um dia são as suas
verificações falhadas vezes o intervalo de verificação. Para outros
monitores (SNMP, agente, Proxmox…) é a fração de intervalos de
cinco minutos em que o monitor reportou pelo menos uma vez
(`dumbmonit_up`), contada a partir da primeira medição — um
monitor adicionado ontem não está «em baixo» pelos 89 dias
anteriores — e a indisponibilidade de um dia são os seus intervalos
silenciosos.

## 3. Anunciar incidentes e manutenção {#3-announce-incidents-and-maintenance}

Em **Settings → Status pages**, sob a lista de páginas, **Incidents and
maintenance** → **New announcement** (também ++ctrl+k++ → *Announce an
incident*).

- Um **incident** tem um título, um impacto (**minor** mostra a página
  como degradada, **major** como uma indisponibilidade) e passa por
  *Investigating → Identified → Monitoring → Resolved*. Publique
  atualizações conforme avança: cada uma transporta um estado e uma
  mensagem, e torna-se a linha mais recente que os visitantes lêem.
  **Resolve** fecha-o com uma mensagem final; depois move-se para
  «Past incidents» durante 30 dias.
- Uma janela de **maintenance** tem um início e um fim e passa por
  *Scheduled → In progress → Completed*. Enquanto está em curso, a
  faixa diz «Scheduled maintenance» e os serviços em baixo mostram-se
  como *Maintenance*.

Uma [janela de manutenção agendada em Alerts](../alerting/maintenance.md)
faz a mesma coisa para o único monitor que cobre, sem um anúncio:
esse serviço lê-se *Maintenance* em vez de vermelho enquanto a janela
está aberta, e o estado geral da página diz *Maintenance* quando nada
mais está em baixo ou degradado. Só as janelas que nomeiam um
monitor aparecem desta forma; o nome e o comentário da janela
permanecem privados.

Um anúncio é mostrado numa página ou em **todas as páginas**.

### Subscritores por email {#email-subscribers}

Os visitantes podem subscrever aos seus anúncios por email uma vez
escolher um **canal de email (SMTP)** no editor, sob **Email
subscribers**. O canal é um que já configurou em
[Notifications](../alerting/notifications.md); o seu servidor e
remetente são reutilizados, os seus próprios destinatários não. Sem
um, a página só oferece o seu feed RSS.

- **Opt-in duplo.** Um visitante introduz um endereço; o DumbMonit
  envia uma ligação de confirmação por email, e nada mais é enviado
  até ser seguida. Um pedido nunca confirmado é esquecido depois de
  dois dias. O formulário responde a mesma frase quer o endereço seja
  novo, pendente ou já subscrito, por isso não diz a ninguém quem
  segue a sua página.
- **O que é enviado.** Um email quando um anúncio é criado, quando uma
  atualização é publicada, e quando o seu estado muda — tanto para
  incidentes como para manutenção, nessa página (ou em toda página,
  para um anúncio mostrado em todas).
- **Cancelar subscrição.** Todo email termina com uma ligação para
  cancelar a subscrição, e transporta os cabeçalhos
  `List-Unsubscribe` e `List-Unsubscribe-Post`, para que o Gmail, o
  Apple Mail e outros ofereçam o seu próprio botão de um clique.
- **Limites.** O endpoint público de subscrição aceita cinco pedidos
  por endereço de cliente e sessenta no total por 15 minutos, e um
  endereço pendente não recebe email duas vezes dentro de dez
  minutos: a página não pode ser transformada num canhão de email.
- O editor lista os subscritores (confirmados ou pendentes) e remove
  qualquer um deles.

As ligações nestes emails apontam para o
[domínio personalizado](#custom-domain) da página quando tem um
(`https://status.example.com/`). Caso contrário são construídas a
partir do **URL público** da
[política de notificação](../alerting/notifications.md) (ou
`DUMBMONIT_PUBLIC_URL`); sem um, a partir do endereço que usou quando
guardou a página por último — nunca a partir do pedido de um
visitante.

## 4. Partilhar a ligação {#4-share-the-link}

A página vive em `https://o-seu-servidor/s/<address>`. Tudo sob `/s/`
e `/api/public/` é servido sem autenticação; o resto do DumbMonit
permanece atrás do início de sessão.

O painel **Share** do editor constrói os trechos abaixo por si, com
uma pré-visualização.

### Emblemas {#badges}

Toda página publicada serve emblemas SVG — planos, com texto branco
sobre cor sólida, legíveis tanto no tema claro como no escuro de um
README do GitHub:

| Emblema | URL |
| --- | --- |
| Estado da página | `/api/public/status/<address>/badge.svg` |
| Disponibilidade da página | `/api/public/status/<address>/uptime.svg?days=30` |
| Tempo de resposta da página | `/api/public/status/<address>/response.svg` |
| Estado do serviço | `/api/public/status/<address>/components/<service>/badge.svg` |
| Disponibilidade do serviço | `/api/public/status/<address>/components/<service>/uptime.svg?days=30` |
| Tempo de resposta do serviço | `/api/public/status/<address>/components/<service>/response.svg` |

`<service>` é o rótulo do serviço em forma de URL: «Public API»
torna-se `public-api` (um segundo «Public API» seria `public-api-2`).
É também a `key` de cada serviço no documento JSON. `days` é `1`, `7`,
`30` (predefinição) ou `90`, e nunca mais do que o histórico da
página: uma página de 30 dias recusa um emblema de 90 dias. A
disponibilidade e tempo de resposta da página são a média sobre os
seus serviços. Um serviço sem nada medido diz **no data**, em cinza.

Num README:

```markdown
[![Status](https://your-server/api/public/status/home-lab/badge.svg)](https://your-server/s/home-lab)
[![Uptime](https://your-server/api/public/status/home-lab/uptime.svg?days=30)](https://your-server/s/home-lab)
[![API](https://your-server/api/public/status/home-lab/components/public-api/uptime.svg)](https://your-server/s/home-lab)
```

Os emblemas ficam em cache por um minuto
(`Cache-Control: public, max-age=60`), para que o proxy de imagens do
GitHub e os seus visitantes não custem nada à base de dados de séries
temporais.

### Embed {#embed}

`/s/<address>/embed` é uma vista compacta para um iframe: o estado
geral e uma linha por serviço com o seu estado e 30 dias de
histórico, e uma ligação para a página completa num novo separador.

```html
<iframe src="https://your-server/s/home-lab/embed"
        title="Home lab status" width="100%" height="320" style="border:0"></iframe>
```

Adicione `?theme=light` ou `?theme=dark` para corresponder à página
anfitriã, e `?history=0` para esconder as barras. A página de estado e
o seu embed são os únicos endereços do DumbMonit que uma página de
terceiros pode colocar numa frame; tudo o resto (incluindo as páginas
de confirmação e cancelamento de subscrição) responde com
`frame-ancestors 'none'` e `X-Frame-Options: DENY`.

### Domínio personalizado {#custom-domain}

Uma página pode ter o seu próprio endereço, como `status.example.com`,
da forma que o Uptime Kuma faz. Escreva o nome no campo **Public
domain** do editor (o **i** junto a ele resume os passos) e guarde.
Quando um pedido alcança o DumbMonit com esse nome no seu cabeçalho
`Host`, o DumbMonit mostra a página na raiz `/`, e nada mais: nesse
nome o ecrã de início de sessão, as definições, a API, os endpoints de
agente, as métricas e as outras páginas de estado todos respondem
`404 Not found`, mesmo para alguém com sessão iniciada, e nenhum
cookie de sessão é lido ou definido. Só a página, o seu embed, as suas
páginas de confirmação e cancelamento de subscrição, os seus
emblemas, feed e documento JSON, e os próprios ficheiros da interface
(`/_app/`, ícones) são servidos.

O nome é guardado em minúsculas, sem `https://` ou uma barra final;
uma porta, um caminho ou um endereço IP são recusados. Duas páginas
não podem partilhar um nome, e o endereço que usa para alcançar o
próprio DumbMonit (ou o seu URL público) não pode ser o domínio de uma
página — guardá-lo bloquear-lhe-ia o acesso. Limpe o campo para
remover o domínio; a página fica em `/s/<address>` de qualquer forma.

Para configurá-lo:

1. **DNS.** Crie o registo para o nome (ou o nome de anfitrião público
   de um Cloudflare Tunnel), apontando para o seu proxy inverso.
2. **Proxy.** Envie o nome para o endereço do DumbMonit, por exemplo
   `http://dumbmonit:8080`, **sem reescrever o caminho**: o proxy
   encaminha `/`, não `/s/<address>`.
3. **Cabeçalho Host.** Passe o cabeçalho `Host` original: é como o
   DumbMonit distingue o domínio da página do seu próprio endereço. O
   DumbMonit só lê `Host`; `X-Forwarded-Host` só conta de um proxy
   listado em `DUMBMONIT_TRUSTED_PROXIES`.
4. **HTTPS** é tratado pelo proxy. O DumbMonit liga a
   `https://<domain>/` nos emails de subscritor, no feed RSS e no
   painel **Share** do editor.

Traefik (etiquetas Docker) — o cabeçalho `Host` é passado por
predefinição:

```yaml
labels:
  - traefik.enable=true
  - traefik.http.routers.status.rule=Host(`status.example.com`)
  - traefik.http.routers.status.entrypoints=websecure
  - traefik.http.routers.status.tls.certresolver=letsencrypt
  - traefik.http.services.status.loadbalancer.server.port=8080
```

Nginx Proxy Manager — adicione um **Proxy Host** com o domínio
`status.example.com`, esquema `http`, nome de anfitrião de
encaminhamento `dumbmonit`, porta `8080`, e um certificado SSL. Nada
mais: passa `Host` por predefinição. Deixe **Custom locations** vazio.

Cloudflare Tunnel — adicione um **Public hostname**
`status.example.com` com o serviço `http://dumbmonit:8080` e sem
caminho. O tunnel mantém o `Host` do visitante; deixe **HTTP Host
Header** nas definições avançadas vazio.

Caddy — `reverse_proxy` passa `Host` e o Caddy obtém o certificado:

```caddyfile
status.example.com {
    reverse_proxy dumbmonit:8080
}
```

Nginx — `proxy_pass` envia o nome do upstream a menos que seja dito o
contrário, por isso defina o cabeçalho:

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

**Resolução de problemas.**

- *O domínio mostra o ecrã de início de sessão ou o painel do
  DumbMonit*: o cabeçalho `Host` não alcançou o DumbMonit — o proxy
  substituiu-o pelo nome do upstream (`dumbmonit:8080`). Adicione
  `proxy_set_header Host $host;` (Nginx), remova um **HTTP Host
  Header** personalizado (Cloudflare Tunnel), ou remova um middleware
  de reescrita de `Host` (Traefik). Nada privado é exposto: o painel
  continua a pedir uma senha.
- *Um `Not found` simples no domínio*: o proxy reescreve o caminho
  (envia `/s/<address>` ou adiciona um prefixo), ou o domínio no
  editor não corresponde exatamente ao nome no navegador. Encaminhe o
  caminho sem o tocar.
- *A página diz que não existe*: a página ainda é um rascunho —
  publique-a.
- *Já não consegue alcançar a interface do DumbMonit*: está a navegar
  através do domínio da página. Use o próprio endereço do DumbMonit,
  ou limpe o **Public domain** da página.

### Atrás de um proxy inverso {#behind-a-reverse-proxy}

Um [domínio personalizado](#custom-domain) é a forma mais simples de
expor só a página de estado. Se preferir manter o caminho
`/s/<address>` num nome público, filtre os caminhos no proxy em vez
disso. Com o Caddy, por exemplo:

```caddyfile
status.example.com {
    @public path /s/* /api/public/* /_app/* /favicon.svg
    reverse_proxy @public dumbmonit:8080
    respond 404
}
```

Com o nginx:

```nginx
location ~ ^/(s/|api/public/|_app/|favicon\.svg) {
    proxy_pass http://dumbmonit:8080;
    proxy_set_header Host $host;
    proxy_set_header X-Forwarded-Proto $scheme;
}
location / { return 404; }
```

`/_app/` transporta os scripts e estilos da interface: a página
precisa deles para renderizar. Encaminhe `X-Forwarded-Proto` e
`X-Forwarded-Host` para que o feed RSS ligue ao endereço certo, e
defina o URL público (ver acima) para que os emails de subscritor
também o façam.

### Feed e documento {#feed-and-document}

- `GET /api/public/status/<address>/rss` — um feed RSS de incidentes e
  janelas de manutenção, um item por anúncio com a sua atualização
  mais recente.
- `GET /api/public/status/<address>` — o documento JSON a partir do
  qual a página é construída, se quiser construir a sua própria: o
  título, descrição e aspeto da página, o estado geral, o `key`,
  `label`, `state`, disponibilidade (`uptime_24h`, `uptime_7d`,
  `uptime_30d`, `uptime_90d`), `latency_ms` e `history` (`date`,
  `uptime_pct`, `down_minutes`, `incidents` por dia) de cada serviço,
  e os anúncios.

Qualquer site pode ler esse documento a partir do navegador (é
enviado com `Access-Control-Allow-Origin: *` e sem cookies).
`GET /api/public/status/<address>/banner.js` é um script já feito que
o transforma numa faixa: ver [Faixa para o site](../integrations/website-banner.md).

O documento fica em cache por 30 segundos no servidor; os emblemas, o
feed e o embed todos lêem esse mesmo documento em cache, por isso
nenhum deles pode dizer mais do que a página.
