# Serviços: HTTP, TCP, DNS, ping, TLS, NTP, SMTP, SQL, MQTT, WebSocket {#services-http-tcp-dns-ping-tls-ntp-smtp-sql-mqtt-websocket}

Onze monitores vigiam **serviços** em vez de equipamento, ao estilo
Uptime Kuma: uma página web, uma porta, um nome DNS, um anfitrião, um
certificado, um servidor de hora, um relay de email, uma base de
dados, um broker MQTT, um endpoint WebSocket. Cada serviço recebe uma
barra de histórico, um tempo de resposta e uma percentagem de
disponibilidade na sua página de dispositivo.

Os últimos cinco vão além de abrir uma ligação: iniciam uma sessão
real no próprio protocolo do serviço. É a diferença entre «a porta
5432 está aberta» e «a base de dados ainda aceita a minha conta e
responde a uma consulta». O NTP fica entre os dois: um único pedido e
resposta, no próprio protocolo NTP, suficiente para ler o relógio do
servidor sem uma sessão.

## Como funciona o estado de serviço {#how-service-state-works}

Um serviço pode responder e continuar em baixo: um `500`, uma
palavra-chave em falta, um certificado expirado. Por isso estes
monitores escrevem um ponto em cada verificação, `probe_success`
sendo 1 ou 0, em vez de ficar silencioso como um dispositivo de
hardware. É isso que torna possível uma percentagem de
disponibilidade: `avg_over_time(dumbmonit_probe_success[30d])`.

| Situação | O que acontece |
|---|---|
| Opção ou endereço inválido, sockets ICMP não permitidos (ver Ping) | Erro de configuração: nada é escrito, a página do dispositivo mostra o erro, nenhum alerta «down». |
| Ligação recusada, tempo limite, TLS recusado, `500`, palavra-chave ausente, perda total de pacotes, senha recusada, consulta falhada | `probe_success = 0`: a indisponibilidade é registada, datada e contada. |
| Tudo bem | `probe_success = 1`. |

Todo monitor tem o seu próprio tempo limite (`timeout_seconds`, 5 s
por predefinição, 60 s no máximo), mais curto do que o
`DUMBMONIT_PROBE_TIMEOUT_SECS` do servidor: interrompido pelo
escalonador, não conseguiria escrever o seu zero.

Regras incorporadas que se aplicam a todos os onze: Service down (3
minutos), Service flapping (mais de seis alterações de estado em
trinta minutos), Slow service (mais de 3 s durante dez minutos). Para
toda verificação que lê um certificado — `http`, `tls`, `smtp`,
`mqtt`, `websocket`: Certificate expiring soon (14 dias) e Certificate
expired.

Uma senha recusada nunca é uma indisponibilidade do serviço: é
registada como `reason="auth"`, para que o alerta diga o que
corrigir.

### Métricas {#metrics}

Todas gauges, todas com o prefixo `dumbmonit_`, todas etiquetadas
`probe="http|tcp|dns|ping|tls|smtp|postgres|mysql|mqtt|websocket"`
além de `target`, `host` e `tag_*`.

| Métrica | Monitores | Significado |
|---|---|---|
| `probe_success` | todos | 1 se o serviço responde corretamente, 0 caso contrário. |
| `probe_duration_seconds` | todos | Duração total da verificação. |
| `probe_failure_info` | todos | Presença (1) com uma etiqueta `reason`: `dns`, `connect`, `timeout`, `tls`, `cert_expired`, `status`, `keyword`, `json`, `body`, `packet_loss`, `record`, `auth`, `protocol`, `query`, `payload`. |
| `probe_http_status_code` | http, websocket | Código de estado obtido. |
| `probe_http_first_byte_seconds` | http | Tempo até aos cabeçalhos de resposta. |
| `probe_http_content_bytes` | http | Tamanho do corpo. |
| `probe_connect_seconds` | todos exceto dns e ping | Estabelecimento de ligação (início de sessão incluído para bases de dados). |
| `probe_tls_handshake_seconds` | tls, http, smtp, mqtt, websocket | Negociação TLS isolada. |
| `probe_tls_version_info` | tls, http, smtp, mqtt, websocket | Presença; etiqueta `version`. |
| `probe_ssl_cert_expiry_days` | tls, http, smtp, mqtt, websocket | Dias até à expiração, negativo se expirado. |
| `probe_ssl_cert_valid` | tls, http, smtp, mqtt, websocket | 1 se a cadeia leva a uma autoridade conhecida. |
| `probe_ssl_cert_issuer_info` | tls, http, smtp, mqtt, websocket | Presença; etiqueta `issuer`. |
| `probe_dns_lookup_seconds` | dns | Tempo de resolução. |
| `probe_dns_answer_records` | dns | Registos do tipo pedido. |
| `probe_dns_answer_fingerprint` | dns | Com «Alert when the answer changes»: uma impressão digital de 32 bits da resposta ordenada. O seu valor não significa nada; só as suas alterações. |
| `probe_icmp_rtt_seconds`, `_min_seconds`, `_max_seconds` | ping | Ida e volta média, mais curta e mais longa. |
| `probe_icmp_packet_loss_ratio` | ping | Perda entre 0 e 1. |
| `probe_icmp_packets_sent`, `probe_icmp_packets_received` | ping | Ecos enviados e recebidos. |
| `probe_smtp_greeting_seconds` | smtp | Tempo até ao banner `220`. |
| `probe_smtp_ehlo_seconds` | smtp | Ida e volta do comando `EHLO`. |
| `probe_smtp_capabilities` | smtp | Extensões anunciadas. |
| `probe_smtp_authenticated` | smtp | 1 quando `AUTH` teve sucesso. |
| `probe_sql_query_seconds` | postgres, mysql | Execução de consulta isolada. |
| `probe_sql_rows` | postgres, mysql | Linhas devolvidas. |
| `probe_sql_value` | postgres, mysql | Primeira coluna da primeira linha, quando é um número. |
| `probe_mqtt_connack_seconds` | mqtt | Tempo até ao `CONNACK`. |
| `probe_mqtt_suback_seconds` | mqtt | Tempo até ao `SUBACK`. |
| `probe_mqtt_message_bytes` | mqtt | Tamanho da mensagem retida. |
| `probe_mqtt_message_value` | mqtt | O seu conteúdo, quando é um número. |
| `probe_ws_handshake_seconds` | websocket | Handshake de abertura. |
| `probe_ws_message_bytes` | websocket | Tamanho da frame recebida. |

## HTTP {#http}

**Site ou API web (HTTP)** — verifica que uma página ou uma API
responde, com o código de estado certo, o conteúdo certo, e um
certificado válido. Exemplos: um site, uma página de saúde de
aplicação, uma API REST, uma interface de serviço autoalojado.

### Configuração {#setup}

1. No endereço, cole o URL completo da página a monitorizar. Sem «http://» ou «https://», HTTPS é assumido.
2. Prefira uma página ligeira que não precise de início de sessão, por exemplo a página de saúde da aplicação («/health», «/status»), em vez da página inicial.
3. Se a página exigir autenticação, preencha a credencial: um nome de utilizador e senha dão autenticação básica, um token é enviado como «Bearer».
4. Por predefinição, qualquer código de estado entre 200 e 299 está bem. Para ir mais além, exija uma palavra-chave na página ou um valor específico numa resposta JSON.
5. Via HTTPS, o certificado é lido automaticamente: será avisado antes de expirar.

!!! warning
    Nunca escreva uma senha ou um token nas opções: são copiados em texto simples em cada medição. Use o campo de credencial, que é encriptado.

Credenciais: nenhuma, nome de utilizador / senha (autenticação
básica) ou token de API (`Authorization: Bearer`).

### Opções {#options}

| Chave | Rótulo | Predefinição | Ajuda |
|---|---|---|---|
| `method` | HTTP method | `GET` | GET serve quase todo caso. HEAD evita descarregar a página quando só o código de estado importa. Escolhas: GET, HEAD, POST, PUT, PATCH, DELETE, OPTIONS. |
| `accepted_status` | Accepted status codes | `200-299` | Códigos ou intervalos considerados normais, separados por vírgulas. Qualquer outro código conta como em baixo. Exemplo: `200-299,301,404`. |
| `keyword` | Expected keyword | *(vazio)* | Texto que tem de aparecer na página. Se estiver em falta, o serviço é reportado em baixo mesmo que a página responda. |
| `keyword_absent` | Keyword must be absent | `false` | Inverte a verificação: a presença da palavra-chave sinaliza uma falha. Útil para uma página de erro que responde 200. |
| `keyword_case_sensitive` | Match keyword case | `false` | Por predefinição, maiúsculas e minúsculas são tratadas da mesma forma. |
| `json_path` | JSON path to check | *(vazio)* | Para uma resposta JSON: caminho do valor a verificar. Preencha-o junto com o valor esperado, nunca um sem o outro. Exemplo: `$.status`. |
| `json_expect` | Expected JSON value | *(vazio)* | Valor que o caminho JSON acima tem de ter, por exemplo «ok» ou «true». |
| `headers` | Extra headers | *(vazio)* | Um ou mais cabeçalhos «Name: value», separados por «\|». Não coloque segredos aqui: as opções são visíveis nos gráficos. |
| `body` | Request body | *(vazio)* | Conteúdo enviado com o pedido, para métodos que esperam um (POST, PUT…). |
| `follow_redirects` | Follow redirects | `true` | Desative para monitorizar o próprio redirecionamento, por exemplo um 301 para HTTPS. |
| `max_redirects` | Maximum redirects followed | `10` | Entre 1 e 20. Sem efeito se os redirecionamentos não são seguidos. |
| `insecure_tls` | Accept an unverifiable certificate | `false` | A verificação já não falha num certificado autoassinado ou emitido por uma autoridade privada. |
| `allow_private_targets` | Allow loopback and link-local targets | `false` | Por predefinição a verificação recusa endereços que só o próprio anfitrião do DumbMonit pode alcançar (127.0.0.1, ::1, 169.254.x.x). Ative isto para monitorizar um serviço a correr no anfitrião do DumbMonit. Endereços de LAN privada (10.x, 192.168.x) são sempre permitidos. |
| `check_certificate` | Read the certificate | `true` | Via HTTPS, também regista a data de expiração do certificado para que seja avisado antes de expirar. |
| `max_body_bytes` | Maximum bytes read | `524288` | Além disto, o resto da página não é descarregado. A palavra-chave e o caminho JSON só são procurados nesta parte. |
| `user_agent` | Announced identity (User-Agent) | `DumbMonit/<version>` | Nome que a verificação dá ao servidor. Altere-o se o servidor filtra robôs. |
| `timeout_seconds` | Timeout (seconds) | `5` | Tempo depois do qual o serviço é reportado em baixo se não respondeu. Entre 1 e 60. |

## TCP {#tcp}

**Porta de rede (TCP)** — verifica que uma porta aceita ligações: SSH,
base de dados, partilha de ficheiros, servidor de jogos, impressora
de rede.

### Configuração {#setup}

1. No endereço, escreva o anfitrião seguido da porta, separados por dois pontos: «nas.home.lan:22» ou «192.168.1.5:445».
2. Para um endereço IPv6, coloque-o entre parênteses retos: «[fd00::1]:445».
3. Pode também deixar o endereço sem porta e introduzi-la na opção «Port»: um dos dois é obrigatório.
4. Só a abertura da ligação é testada: nenhum dado é enviado ao serviço, por isso não tem nenhum efeito nele.

!!! warning
    Uma porta aberta não prova que a aplicação por detrás funciona. Para um serviço web, prefira o tipo «Site ou API web», que lê a resposta.

### Opções {#options_2}

| Chave | Rótulo | Predefinição | Ajuda |
|---|---|---|---|
| `port` | Port | *(vazio)* | Porta a abrir, se o endereço ainda não a der como «anfitrião:porta». Um dos dois é obrigatório. |
| `allow_private_targets` | Allow loopback and link-local targets | `false` | Por predefinição a verificação recusa endereços que só o próprio anfitrião do DumbMonit pode alcançar (127.0.0.1, ::1, 169.254.x.x). Ative isto para monitorizar um serviço a correr no anfitrião do DumbMonit. Endereços de LAN privada (10.x, 192.168.x) são sempre permitidos. |
| `timeout_seconds` | Timeout (seconds) | `5` | Tempo depois do qual o serviço é reportado em baixo se não respondeu. Entre 1 e 60. |

## DNS {#dns}

**Nome de domínio (DNS)** — verifica que um nome resolve, e que
aponta para o endereço certo. Exemplos: o seu nome de domínio, um
nome interno servido pelo seu Pi-hole ou AdGuard, um registo MX.

### Configuração {#setup_2}

1. No endereço, escreva o nome a resolver, sem «http://»: «www.example.com».
2. Por predefinição, a verificação pede ao resolvedor do sistema um endereço IPv4 (registo A). Escolha outro tipo de registo nas opções se precisar.
3. Para monitorizar o seu próprio servidor DNS, introduza o seu endereço IP em «DNS server to query»: a verificação falhará se deixar de responder.
4. Para detetar sequestro ou uma má configuração, liste em «Expected values» os endereços que a resposta tem de conter.

!!! warning
    O servidor DNS é dado pelo seu endereço IP, não por um nome: precisaria de um resolvedor para resolver o resolvedor.

### Opções {#options_3}

| Chave | Rótulo | Predefinição | Ajuda |
|---|---|---|---|
| `record_type` | Record type | `A` | A para um endereço IPv4, AAAA para IPv6, MX para email, CNAME para um alias… Escolhas: A, AAAA, CNAME, MX, TXT, NS, SOA, SRV, PTR, CAA. |
| `resolver` | DNS server to query | *(vazio)* | Endereço IP de um resolvedor, porta opcional. Vazio: o resolvedor do sistema. Útil para verificar o seu próprio servidor DNS. Exemplo: `1.1.1.1` ou `10.0.0.1:5353`. |
| `expect` | Expected values | *(vazio)* | Valores que todos têm de aparecer na resposta, separados por vírgulas. Vazio: só a resolução é verificada. |
| `expect_mode` | Comparison | `contains` | `contains`: cada valor esperado tem de aparecer em algum lugar da resposta. `exact`: a resposta tem de ter esses valores e nada mais. |
| `forbid` | Forbidden values | *(vazio)* | Valores que nunca devem aparecer na resposta, separados por vírgulas. Verificado antes dos esperados. |
| `alert_on_change` | Alert when the answer changes | `false` | Regista uma impressão digital da resposta ordenada e alerta sempre que muda, sem ter de escrever os valores esperados. |
| `timeout_seconds` | Timeout (seconds) | `5` | Tempo depois do qual o serviço é reportado em baixo se não respondeu. Entre 1 e 60. |

### Afirmar o que uma zona responde {#asserting-what-a-zone-answers}

`expect` por si só apanha o registo que desapareceu. Não apanha o
registo que foi *adicionado*: uma zona sequestrada frequentemente
mantém o `A` legítimo e coloca um segundo ao lado, e um cliente então
alcança qualquer um dos dois aleatoriamente.

- `expect_mode = exact` recusa qualquer valor que a expectativa não
  cubra. A ordem não importa — o DNS não mantém uma.
- `forbid` nomeia valores que nunca devem voltar: o endereço de um
  antigo anfitrião, um `TXT` de validação expirado, um servidor de
  email que desativou. É verificado primeiro, por isso ganha sobre
  `expect`.

Ambos comparam sem distinguir maiúsculas de minúsculas, ignoram o
ponto final de um nome e as aspas em volta de um `TXT`, e tratam um
valor esperado como encontrado quando aparece dentro de um registo —
por isso um `MX` corresponde a `mail.example.com` sem ter de copiar a
sua prioridade.

### Alertar quando a resposta muda {#alerting-when-the-answer-changes}

Escrever os valores esperados não é sempre prático: um nome servido
por uma CDN, registos que outra pessoa gere, ou simplesmente uma zona
sobre a qual quer ser avisado sempre que se mover. Marque **Alert
when the answer changes**: toda verificação então regista uma
impressão digital da resposta, e a regra incorporada
[DNS answer changed](../alerting/rules.md#reverse-proxies-and-domains)
dispara quando difere da anterior. O alerta resolve-se por si só uma
hora depois; a nova resposta é a referência a partir daí.

A impressão digital cobre os valores da resposta, ordenados e sem
duplicados, com a cadeia CNAME incluída: um round-robin que devolve
os mesmos endereços noutra ordem não a altera, nem um TTL. Um nome
cuja resposta muda em cada consulta (uma grande pool como
`pool.ntp.org`, ou DNS geográfico que responde de forma diferente de
um resolvedor para o outro) alertaria o tempo todo: deixe a opção
desligada para esses, ou consulte um resolvedor fixo.

## Ping {#ping}

**Anfitrião alcançável (ping)** — envia alguns ecos ICMP e mede o
tempo de resposta e a perda de pacotes. Exemplos: router ou gateway
doméstico, ponto de acesso Wi-Fi, impressora, máquina sem agente ou
SNMP, anfitrião remoto.

### Configuração {#setup_3}

1. No endereço, escreva o nome de anfitrião ou endereço IP: «192.168.1.1» ou «router.home.lan».
2. Em cada sondagem, a verificação envia quatro ecos e mede o tempo de resposta, juntamente com a fração de pacotes perdidos.
3. Por predefinição, só uma perda total conta como em baixo; a perda parcial continua visível nos gráficos. Baixe «Tolerated loss» para ser avisado mais cedo.
4. Se o DumbMonit corre em Docker, mantenha as linhas `sysctls:` do docker-compose.yml incluído sob o serviço dumbmonit: permitem que o utilizador sem privilégios do contentor abra sockets de eco ICMP. Nenhuma capacidade é necessária.

!!! warning
    Sem esse sysctl, a verificação não consegue abrir um socket ICMP: reporta isto como um erro de configuração, não como uma falha do anfitrião. Alguns dispositivos também ignoram pings de propósito: verifique isso antes de tirar conclusões.

```yaml
services:
  dumbmonit:
    sysctls:
      net.ipv4.ping_group_range: "0 2147483647"
```

Fora do Docker, `sysctl -w net.ipv4.ping_group_range="0 2147483647"`
no anfitrião faz o mesmo, e um socket raw (`NET_RAW`, root) também
funciona.

### Opções {#options_4}

| Chave | Rótulo | Predefinição | Ajuda |
|---|---|---|---|
| `count` | Number of echoes | `4` | Pacotes enviados em cada sondagem, de 1 a 20. |
| `packet_timeout_ms` | Wait per packet (ms) | `1000` | Tempo permitido para cada resposta, de 50 a 10000 ms. |
| `interval_ms` | Gap between two echoes (ms) | `100` | Pausa entre dois ecos, de 0 a 5000 ms. |
| `payload_bytes` | Payload size (bytes) | `56` | Dados transportados por cada eco, de 0 a 1400 bytes. |
| `ip_version` | IP version | `auto` | «auto» toma o primeiro endereço resolvido; force 4 ou 6 se o anfitrião tiver ambos e um não responder. Escolhas: auto, 4, 6. |
| `loss_threshold_percent` | Tolerated loss (%) | `100` | Acima desta percentagem de pacotes perdidos, o anfitrião é reportado em baixo. 100: só uma perda total conta, a perda parcial continua visível nos gráficos. |
| `timeout_seconds` | Timeout (seconds) | `5` | Orçamento total para a sondagem, de 1 a 60. Recusado se não cobre «número de ecos × espera por pacote». |

## TLS {#tls}

**Certificado TLS** — verifica que um certificado é válido e avisa
antes de expirar, em qualquer porta encriptada. Exemplos: servidor de
email (IMAPS, SMTPS), proxy inverso, diretório LDAPS, broker MQTT,
interface de administração.

### Configuração {#setup_4}

1. No endereço, escreva o anfitrião e, se não for 443, a porta do serviço encriptado: «mail.example.com:993», «ldap.home.lan:636».
2. A verificação abre uma ligação encriptada, lê o certificado apresentado e fecha de imediato: nenhum dado é trocado com a aplicação.
3. Regista o número de dias até à expiração, o emissor e a versão TLS. As regras de alerta incluídas avisam a catorze dias, depois na expiração.
4. Se o serviço for alcançado pelo seu endereço IP mas o certificado transportar um nome, introduza esse nome em «Server name (SNI)».
5. Para um certificado emitido pela sua própria autoridade, marque «Accept an unverifiable certificate»: a data de expiração continua a ser monitorizada.

!!! warning
    Para um site web, o tipo «Site ou API web» já lê o certificado: este tipo é destinado a serviços que não falam HTTP, ou cuja aplicação não quer atingir.

O handshake completa-se mesmo quando o certificado é recusado: é isso
que permite ler a data de expiração de um certificado que já
expirou. O veredicto da verificação está em `probe_ssl_cert_valid`.

### Opções {#options_5}

| Chave | Rótulo | Predefinição | Ajuda |
|---|---|---|---|
| `server_name` | Server name (SNI) | *(vazio)* | Nome de domínio anunciado ao servidor e verificado no certificado. Defina-o quando o endereço é um IP atrás de um proxy inverso. |
| `insecure_tls` | Accept an unverifiable certificate | `false` | Uma cadeia não verificável já não conta como uma falha: a data de expiração continua a ser registada. |
| `allow_private_targets` | Allow loopback and link-local targets | `false` | Por predefinição a verificação recusa endereços que só o próprio anfitrião do DumbMonit pode alcançar (127.0.0.1, ::1, 169.254.x.x). Ative isto para monitorizar um serviço a correr no anfitrião do DumbMonit. Endereços de LAN privada (10.x, 192.168.x) são sempre permitidos. |
| `timeout_seconds` | Timeout (seconds) | `5` | Tempo depois do qual o serviço é reportado em baixo se não respondeu. Entre 1 e 60. |

## SMTP {#smtp}

**Relay de email (SMTP)** — abre uma sessão real num servidor de
email: saudação, EHLO, STARTTLS, e o início de sessão se der um.
Exemplos: o relay SMTP do seu fornecedor, um Postfix ou msmtp local,
o Proxmox Mail Gateway, a porta de submissão de um servidor de email.

### Configuração {#setup_5}

1. No endereço, escreva o servidor de email, e a porta se não for a habitual: «smtp.example.com» ou «smtp.example.com:2525».
2. Escolha a encriptação que o seu relay usa: STARTTLS para a porta de submissão 587, TLS para a porta 465, None para um relay na sua própria rede a escutar na porta 25.
3. Para verificar que o relay ainda aceita a sua conta, preencha a credencial: a verificação então executa AUTH e reporta uma senha recusada como tal, não como uma indisponibilidade.
4. Nada é alguma vez enviado: a verificação para depois da saudação, das extensões e do início de sessão opcional, depois desliga. Nenhuma mensagem entra na fila.
5. Para ser avisado quando uma extensão desaparece, nomeie-a em «Expected extension»: um relay que deixa de anunciar STARTTLS é um relay que enviaria o seu email em texto simples.

!!! warning
    Uma sessão aceite não prova que o email sai. Um relay que aceita tudo e o enfileira para sempre responde perfeitamente aqui: vigie as próprias filas para isso.

Credenciais: nenhuma, ou um nome de utilizador e senha enviados com
`AUTH PLAIN` (ou `AUTH LOGIN` quando o servidor só oferece isso). São
enviados **depois** de a ligação ser encriptada e depois de o
certificado ter sido julgado: um relay cuja cadeia não pode ser
verificada nunca os recebe.

Via TLS ou STARTTLS, o certificado é lido como a verificação TLS o lê
— expiração, emissor, versão de protocolo — por isso um relay de
email também é um certificado sobre o qual é avisado.

### O que não consegue detetar {#what-it-cannot-detect}

Que o email realmente sai. Um relay que aceita toda sessão e depois
adia tudo responde perfeitamente aqui; vigie as suas filas em vez
disso (ver a página do Proxmox Mail Gateway). Também nunca envia uma
mensagem, por isso não diz nada sobre filtragem de destinatário, SPF
ou DKIM.

### Opções {#options_6}

| Chave | Rótulo | Predefinição | Ajuda |
|---|---|---|---|
| `security` | Encryption | `starttls` | STARTTLS começa em texto simples e atualiza (porta 587), TLS encripta desde o primeiro byte (porta 465), None nunca encripta (porta 25) e só convém a um relay na sua própria rede. |
| `port` | Port | *(vazio)* | Usada se o endereço não der uma. Vazio: 587 com STARTTLS, 465 com TLS, 25 sem encriptação. |
| `helo_name` | Name announced (EHLO) | `dumbmonit` | Nome que a verificação dá quando se apresenta. Um relay estrito recusa um nome que não consegue resolver. |
| `expect_capability` | Expected extension | *(vazio)* | Extensão que o servidor tem de anunciar na sua resposta EHLO, por exemplo STARTTLS ou AUTH. Vazio: nenhuma expectativa. |
| `server_name` | Server name (SNI) | *(o anfitrião)* | Nome de domínio anunciado ao servidor e verificado no certificado. Defina-o quando o endereço é um IP. |
| `insecure_tls` | Accept an unverifiable certificate | `false` | Uma cadeia não verificável já não conta como uma falha: a data de expiração continua a ser registada. |
| `allow_private_targets` | Allow loopback and link-local targets | `false` | Ver «Endereços que as verificações recusam» abaixo. |
| `timeout_seconds` | Timeout (seconds) | `5` | Tempo depois do qual o serviço é reportado em baixo se não respondeu. Entre 1 e 60. |

### Motivos de falha {#failure-reasons}

| `reason` | O que aconteceu |
|---|---|
| `connect` | A porta está fechada ou o relay está inacessível. |
| `protocol` | Algo respondeu, mas não SMTP — ou recusou a ligação, `EHLO`, ou o STARTTLS que tinha anunciado. |
| `auth` | A conta foi recusada (`535`, `530`, `534`). O relay está bem; a credencial não. |
| `payload` | A extensão nomeada em «Expected extension» não é anunciada. |
| `tls`, `cert_expired` | Mesmo significado que para a verificação TLS. |

## PostgreSQL {#postgresql}

**Base de dados PostgreSQL** — liga, autentica e executa uma
consulta: o sinal para uma base de dados que está ativa mas já não
responde. Exemplos: a base de dados por detrás do Nextcloud ou do
Immich, um recorder do Home Assistant, uma base de dados de
aplicação, uma instância TimescaleDB.

### Configuração {#setup_6}

1. No servidor, crie uma conta para monitorização e não lhe dê mais do que o direito de ligar: CREATE ROLE dumbmonit LOGIN PASSWORD 'a-long-password';
2. Certifique-se de que essa conta pode alcançar o servidor a partir da máquina DumbMonit, o que geralmente significa mais uma linha em pg_hba.conf seguida de uma recarga de configuração.
3. No endereço, escreva o servidor, e a porta se não for 5432: «db.home.lan» ou «db.home.lan:5433».
4. A consulta predefinida é SELECT 1, que não lê nenhuma tabela: nada mais tem de ser concedido. O tempo de ligação e o tempo de consulta são registados separadamente.
5. Para monitorizar algo próprio, substitua a consulta por uma que devolva um único número, e torna-se um gráfico: o comprimento de uma fila, uma contagem de linhas, um atraso de replicação.

!!! warning
    A verificação abre uma ligação real em cada sondagem. Numa instância já perto do seu limite de ligações, dê-lhe um intervalo mais longo.

Credenciais: um nome de utilizador e senha, obrigatórios. Uma base de
dados nunca é monitorizada anonimamente, por isso o formulário não
oferece mais nada.

### O que não consegue detetar {#what-it-cannot-detect_2}

Atraso de replicação, um bloqueio que se arrasta, uma tabela que
continua a crescer. Tudo isso vive nas próprias vistas de
administração do motor — escreva a consulta você mesmo, e o número
que devolve torna-se `probe_sql_value`, um gráfico sobre o qual pode
alertar.

### Opções {#options_7}

| Chave | Rótulo | Predefinição | Ajuda |
|---|---|---|---|
| `port` | Port | `5432` | Usada se o endereço não der uma. |
| `database` | Database | `postgres` | Base de dados aberta na ligação. A conta de monitorização tem de poder ligar-se a ela. |
| `query` | Query | `SELECT 1` | Executada em cada verificação. A predefinição não lê nenhuma tabela, por isso a conta não precisa de nenhum privilégio além de ligar. Uma consulta que devolve um número torna-a num gráfico. |
| `expect` | Expected value | *(vazio)* | Valor que a primeira coluna da primeira linha tem de ter. Vazio: só o sucesso da consulta é verificado. |
| `sslmode` | Encryption | `prefer` | `prefer` encripta quando o servidor o oferece, `require` recusa ligar sem ele, `verify-full` também verifica o certificado e o nome, `disable` nunca encripta. |
| `allow_private_targets` | Allow loopback and link-local targets | `false` | Ver «Endereços que as verificações recusam» abaixo. |
| `timeout_seconds` | Timeout (seconds) | `5` | Tempo depois do qual o serviço é reportado em baixo se não respondeu. Entre 1 e 60. |

## MySQL e MariaDB {#mysql-and-mariadb}

**Base de dados MySQL ou MariaDB** — liga, autentica e executa uma
consulta: o sinal para uma base de dados que está ativa mas já não
responde. Exemplos: a base de dados por detrás de um WordPress, um
Nextcloud ou um Kimai, uma base de dados de aplicação, um MariaDB em
Docker.

### Configuração {#setup_7}

1. No servidor, crie uma conta para monitorização sem nenhum privilégio: CREATE USER 'dumbmonit'@'%' IDENTIFIED BY 'a-long-password';
2. Verifique que a conta pode ligar-se a partir da máquina DumbMonit: um padrão de anfitrião de localhost só aceitaria ligações feitas no próprio servidor.
3. No endereço, escreva o servidor, e a porta se não for 3306: «db.home.lan» ou «db.home.lan:3307».
4. A consulta predefinida é SELECT 1, que não lê nenhuma tabela: nada mais tem de ser concedido. O tempo de ligação e o tempo de consulta são registados separadamente.
5. Para monitorizar algo próprio, substitua a consulta por uma que devolva um único número, e torna-se um gráfico: o comprimento de uma fila, uma contagem de linhas, um atraso de replicação.

!!! warning
    A verificação abre uma ligação real em cada sondagem. Numa instância já perto do seu limite de ligações, dê-lhe um intervalo mais longo.

### Opções {#options_8}

| Chave | Rótulo | Predefinição | Ajuda |
|---|---|---|---|
| `port` | Port | `3306` | Usada se o endereço não der uma. |
| `database` | Database | *(vazio)* | Base de dados aberta na ligação. Vazio: nenhuma, o que basta para a consulta predefinida. |
| `query` | Query | `SELECT 1` | Executada em cada verificação. A predefinição não lê nenhuma tabela, por isso a conta não precisa de nenhum privilégio além de ligar. Uma consulta que devolve um número torna-a num gráfico. |
| `expect` | Expected value | *(vazio)* | Valor que a primeira coluna da primeira linha tem de ter. Vazio: só o sucesso da consulta é verificado. |
| `sslmode` | Encryption | `prefer` | `prefer` encripta quando o servidor o oferece, `require` recusa ligar sem ele, `verify-full` também verifica o certificado e o nome, `disable` nunca encripta. |
| `allow_private_targets` | Allow loopback and link-local targets | `false` | Ver «Endereços que as verificações recusam» abaixo. |
| `timeout_seconds` | Timeout (seconds) | `5` | Tempo depois do qual o serviço é reportado em baixo se não respondeu. Entre 1 e 60. |

### Motivos de falha, ambos os motores {#failure-reasons-both-engines}

| `reason` | O que aconteceu |
|---|---|
| `connect` | A porta está fechada, o servidor está inacessível, ou recusou a ligação por um motivo que não a conta (a iniciar, demasiadas ligações). |
| `auth` | A conta foi recusada: senha errada, nenhuma linha `pg_hba.conf`, padrão de anfitrião que não cobre a máquina DumbMonit. |
| `query` | Ligou e autenticou, mas a consulta falhou: tabela em falta, base de dados só de leitura, erro de sintaxe. |
| `payload` | A consulta funcionou mas devolveu algo diferente do valor esperado. |
| `timeout` | A ligação ou a consulta não terminou a tempo — o caso «ativo mas lento». |

## MQTT {#mqtt}

**Broker MQTT** — liga ao broker, subscreve a um tópico, e pode
esperar por uma mensagem retida. Exemplos: Mosquitto, o broker por
detrás do Home Assistant, Zigbee2MQTT, sensores ESPHome.

### Configuração {#setup_8}

1. No endereço, escreva o broker, e a porta se não for a habitual: «broker.home.lan» ou «broker.home.lan:1884».
2. Marque «Encrypted connection» para um broker a escutar na 8883, e preencha a credencial se exigir uma conta.
3. Ligar já vale a pena monitorizar: um broker que recusa a sua conta, ou que deixou de responder, é a razão pela qual toda a casa ficou silenciosa.
4. Para ir mais além, nomeie um tópico: a verificação então subscreve-o, e um broker que recusa a subscrição diz-lhe que a conta perdeu os seus direitos de acesso.
5. Marque «Expect a retained message» só para um tópico que transporta uma, tipicamente um tópico de disponibilidade. Um tópico publicado de vez em quando parece silencioso a um cliente que acabou de se ligar.

!!! warning
    A verificação nunca publica nada. Também não consegue dizer quão antiga é uma mensagem retida: o MQTT não as data, por isso um valor congelado há três semanas continua a contar como presente.

### O que não consegue detetar {#what-it-cannot-detect_3}

Que um determinado sensor deixou de emitir. A mensagem retida que a
verificação lê é o que o broker guardou, possivelmente há três
semanas — o MQTT não a data. Para frescura, vigie a automação que a
consome, ou use um heartbeat.

### Opções {#options_9}

| Chave | Rótulo | Predefinição | Ajuda |
|---|---|---|---|
| `tls` | Encrypted connection | `false` | Encripta a sessão desde o primeiro byte, como os brokers fazem na porta 8883. |
| `port` | Port | *(vazio)* | Usada se o endereço não der uma. Vazio: 1883 em texto simples, 8883 encriptada. |
| `topic` | Topic | *(vazio)* | Tópico que a verificação subscreve. Vazio: só liga, o que já lhe diz que o broker está vivo e aceita a sua conta. |
| `expect_message` | Expect a retained message | `false` | Uma mensagem tem de chegar nesse tópico antes do tempo limite. Só mensagens retidas chegam de imediato: um tópico que é apenas publicado de vez em quando parecerá silencioso. |
| `expect` | Expected content | *(vazio)* | Texto que essa mensagem tem de conter. Preenchê-lo implica esperar uma mensagem. |
| `client_id` | Client identifier | *(derivado)* | Nome anunciado ao broker. A predefinição deriva do dispositivo, para que duas verificações nunca se desliguem uma à outra. |
| `server_name` | Server name (SNI) | *(o anfitrião)* | Nome de domínio anunciado ao broker e verificado no certificado. Defina-o quando o endereço é um IP. |
| `insecure_tls` | Accept an unverifiable certificate | `false` | Uma cadeia não verificável já não conta como uma falha: a data de expiração continua a ser registada. |
| `allow_private_targets` | Allow loopback and link-local targets | `false` | Ver «Endereços que as verificações recusam» abaixo. |
| `timeout_seconds` | Timeout (seconds) | `5` | Tempo depois do qual o serviço é reportado em baixo se não respondeu. Entre 1 e 60. |

### Motivos de falha {#failure-reasons_2}

| `reason` | O que aconteceu |
|---|---|
| `connect` | A porta está fechada ou o broker está inacessível. |
| `auth` | O `CONNACK` recusou a conta, ou o broker recusou a subscrição — uma lista de controlo de acesso já não cobre este tópico. |
| `protocol` | Algo respondeu mas não fala MQTT 3.1.1, ou o broker rejeitou o identificador de cliente. |
| `payload` | Nenhuma mensagem retida chegou ao tópico, ou o seu conteúdo não contém o texto esperado. |

## WebSocket {#websocket}

**Endpoint WebSocket** — executa o handshake de atualização, e pode
enviar uma frame e esperar por uma: uma falha diferente de uma página
HTTP simples. Exemplos: a API do Home Assistant, um painel em tempo
real, um fluxo de logs, um endpoint atrás de um proxy inverso.

### Configuração {#setup_9}

1. No endereço, cole o endpoint completo: «wss://home.example.com/api/websocket». Sem esquema, wss é assumido.
2. A verificação executa todo o handshake de abertura e verifica a resposta que o servidor calcula a partir da chave que lhe foi enviada: um proxy inverso que perdeu o cabeçalho Upgrade é apanhado aqui, onde uma verificação HTTP ainda veria um 200 saudável.
3. Nada mais é necessário para a maioria dos endpoints. Um estado diferente de 101 é reportado com o seu código, por isso um 401 de um token expirado não é confundido com uma indisponibilidade.
4. Para ir mais além, preencha «Expected content» com um fragmento da primeira frame que o servidor envia por si próprio, por exemplo a saudação da API do Home Assistant.
5. Se o endpoint não diz nada até lhe falarem, coloque uma mensagem em «Frame to send» e o que a sua resposta tem de conter em «Expected content».

!!! warning
    A verificação abre e desliga dentro de um segundo. Uma ligação que uma firewall ou um tempo limite de proxy corta depois de trinta segundos parece perfeitamente saudável aqui.

Credenciais: nenhuma, um nome de utilizador e senha (enviados como
autenticação HTTP básica no pedido de abertura) ou um token (enviado
como `Authorization: Bearer`).

### Opções {#options_10}

| Chave | Rótulo | Predefinição | Ajuda |
|---|---|---|---|
| `path` | Path | *(do endereço)* | Caminho do pedido de abertura. Tomado do endereço quando o transporta. |
| `port` | Port | *(vazio)* | Usada se o endereço não der uma. Vazio: 443 para wss, 80 para ws. |
| `send` | Frame to send | *(vazio)* | Frame de texto enviada uma vez a ligação aberta. Vazio: nada é enviado. |
| `expect` | Expected content | *(vazio)* | Texto que uma frame recebida tem de conter. Vazio e sem nada a enviar, só o handshake de abertura é verificado. |
| `subprotocol` | Subprotocol | *(vazio)* | Valor de Sec-WebSocket-Protocol. A verificação falha se o servidor escolher um diferente. |
| `origin` | Origin | *(vazio)* | Valor do cabeçalho Origin, que alguns servidores exigem antes de atualizar. |
| `server_name` | Server name (SNI) | *(o anfitrião)* | Nome de domínio anunciado ao servidor e verificado no certificado. Defina-o quando o endereço é um IP. |
| `insecure_tls` | Accept an unverifiable certificate | `false` | Uma cadeia não verificável já não conta como uma falha: a data de expiração continua a ser registada. |
| `allow_private_targets` | Allow loopback and link-local targets | `false` | Ver «Endereços que as verificações recusam» abaixo. |
| `timeout_seconds` | Timeout (seconds) | `5` | Tempo depois do qual o serviço é reportado em baixo se não respondeu. Entre 1 e 60. |

### Motivos de falha {#failure-reasons_3}

| `reason` | O que aconteceu |
|---|---|
| `connect` | A porta está fechada ou o anfitrião está inacessível. |
| `status` | O endpoint respondeu algo diferente de `101`; `probe_http_status_code` transporta o código. |
| `protocol` | Respondeu `101` mas o seu `Sec-WebSocket-Accept` não corresponde à chave enviada, escolheu outro subprotocolo, ou fechou a ligação de imediato. |
| `payload` | Nenhuma frame chegou, ou não contém o texto esperado. |

## NTP {#ntp}

**Servidor de hora NTP** — envia um único pedido SNTP e lê a
resposta: desvio, atraso de ida e volta, stratum e indicador leap.
Exemplos: um router ou NAS a atuar como servidor de hora local, um
servidor NTP interno (chrony, ntpd), um servidor de pool público
usado como referência.

### Configuração {#setup_10}

1. No endereço, escreva o nome ou endereço IP do servidor: «pool.ntp.org» ou «192.168.1.1». A porta UDP 123 é usada a menos que a opção «Port» diga o contrário.
2. A verificação envia um único pedido SNTP e lê a resposta: nenhum estado é mantido entre sondagens, e nada é escrito ao servidor.
3. Regista o desvio entre o relógio do servidor e o anfitrião do DumbMonit, o atraso de ida e volta, o stratum e o indicador leap.
4. Um servidor que responde stratum 16, ou cujo indicador leap lê «unsynchronized», nunca conseguiu sincronizar: a verificação reporta-o em baixo seja qual for o desvio.
5. Baixe «Tolerated offset» para ser avisado mais cedo do que 100 ms; suba-o para um servidor a vários saltos de um relógio de referência.

!!! warning
    O desvio só é tão bom quanto o próprio relógio do anfitrião do DumbMonit: se esse anfitrião não é mantido em sincronia (NTP, a sincronização de hora de um hipervisor), o desvio medido reflete a sua deriva tanto quanto a do servidor.

Credenciais: nenhuma.

### Opções {#options_11}

| Chave | Rótulo | Predefinição | Ajuda |
|---|---|---|---|
| `port` | Port | `123` | Porta UDP do servidor NTP. |
| `offset_threshold_ms` | Tolerated offset (ms) | `100` | Acima deste desvio de relógio, o servidor é reportado em baixo. Um servidor não sincronizado (stratum 16) falha sempre, qualquer que seja este valor. |
| `allow_private_targets` | Allow loopback and link-local targets | `false` | Ver «Endereços que as verificações recusam» abaixo. |
| `timeout_seconds` | Timeout (seconds) | `5` | Tempo depois do qual o serviço é reportado em baixo se não respondeu. Entre 1 e 60. |

### Motivos de falha {#failure-reasons_4}

| `reason` | O que aconteceu |
|---|---|
| `connect` | O socket UDP não pôde ser aberto, ou a resposta veio de um endereço inesperado. |
| `timeout` | Nenhuma resposta chegou dentro do tempo limite. |
| `protocol` | A resposta não era um pacote NTP válido, ou não veio de um servidor (modo errado). |
| `unsynchronized` | O servidor respondeu stratum 16 ou 0 (um «kiss-o'-death», que também nomeia o motivo no registo), ou o seu indicador leap lê «unsynchronized». |
| `clock_offset` | O desvio medido excede «Tolerated offset». |

## Endereços que as verificações recusam {#addresses-the-checks-refuse}

As verificações que ligam a um endereço que lhes dá — HTTP, TCP,
TLS, NTP, SMTP, PostgreSQL, MySQL, MQTT, WebSocket — reportam o que
veem — o que também é, palavra por palavra, o que uma falsificação de
pedido do lado do servidor faz. Para impedir um administrador de
monitorização de ler serviços que só o anfitrião do DumbMonit pode
alcançar (a base de dados de métricas incorporada em
`127.0.0.1:8428`, o serviço de metadados de um fornecedor cloud em
`169.254.169.254`), as verificações recusam por predefinição:

- endereços loopback (`127.0.0.0/8`, `::1`);
- endereços link-local (`169.254.0.0/16`, `fe80::/10`), incluindo o
  endereço de metadados cloud, e o endereço não especificado
  (`0.0.0.0`, `::`).

Os intervalos de LAN privada (`10.0.0.0/8`, `172.16.0.0/12`,
`192.168.0.0/16`, `fc00::/7`) são sempre permitidos: monitorizá-los é
o objetivo de um homelab.

A regra aplica-se ao endereço que escreveu, a todo endereço para o
qual um nome de anfitrião resolve (verificado de novo no momento da
ligação, para que um nome que mude a sua resposta não possa escapar),
e a todo redirecionamento que a verificação HTTP segue. Um endereço
recusado mostra-se como um erro de configuração no destino — nenhum
alerta de «down» é enviado — nomeando a opção a ativar:
`allow_private_targets`. Ative-a para um destino que realmente
monitoriza um serviço no próprio anfitrião do DumbMonit.

Um redirecionamento para um endereço recusado é reportado como uma
medição falhada (`reason="connect"`), porque é o serviço monitorizado
que mudou, não a configuração.

Valores encontrados num `json_path` nunca são repetidos além de 32
caracteres numa mensagem de falha.

## Erros comuns {#common-errors}

| Sintoma | Causa provável |
|---|---|
| O Ping mostra um erro de configuração | O sysctl `net.ipv4.ping_group_range` está em falta no ficheiro Compose. Ver acima. |
| HTTP em baixo com `reason="keyword"` | A palavra-chave só é procurada nos primeiros `max_body_bytes`, sem distinguir maiúsculas a menos que `keyword_case_sensitive` esteja definido. |
| HTTP em baixo com `reason="tls"` | Autoassinado ou autoridade privada: defina `insecure_tls`. |
| Uma verificação mostra um erro de configuração nomeando `allow_private_targets` | O endereço é loopback ou link-local. Ver «Endereços que as verificações recusam» acima. |
| TCP em baixo embora `telnet host port` funcione do seu portátil | O contentor DumbMonit não consegue alcançar o anfitrião (rede Docker, firewall). Teste de dentro da rede do contentor. |
| DNS em baixo com `reason="record"` | A resposta não contém todo valor listado em `expect`. |
| Ping em baixo com `reason="packet_loss"` | Perda acima de `loss_threshold_percent`; alguns dispositivos limitam a taxa de ICMP. |
| DNS em baixo com `reason="record"` embora `expect` corresponda | `expect_mode` é `exact` e a resposta tem um valor extra, ou `forbid` correspondeu. |
| SMTP em baixo com `reason="protocol"` em `security = starttls` | O relay não anuncia STARTTLS nesta porta. Use `tls` para 465, `none` para um relay local simples. |
| Uma verificação de base de dados em baixo com `reason="auth"` | A conta existe mas pode não ligar da máquina DumbMonit: verifique `pg_hba.conf`, ou a parte de anfitrião do utilizador MySQL. |
| Uma verificação de base de dados em baixo com `reason="query"` | A ligação e o início de sessão funcionaram. A base de dados está ativa e a recusar-se a trabalhar: só leitura, tabela em falta, recuperação. |
| MQTT em baixo com `reason="payload"` num tópico que claramente tem tráfego | Só mensagens *retidas* chegam a um cliente que acabou de subscrever. Publique esse tópico com a flag retain, ou desmarque «Expect a retained message». |
| WebSocket em baixo com `reason="protocol"` depois de um `101` | Algo no caminho responde à atualização sem falar WebSocket — geralmente uma regra de proxy inverso que engole o cabeçalho `Upgrade`. |
