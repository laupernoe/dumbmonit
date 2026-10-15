# Serviços: HTTP, TCP, DNS, ping, TLS, NTP, SMTP, SQL, MQTT, WebSocket {#services-http-tcp-dns-ping-tls-ntp-smtp-sql-mqtt-websocket}

Onze monitores vigiam **serviços** em vez de equipamento, ao estilo Uptime Kuma:
uma página web, uma porta, um nome DNS, um anfitrião, um certificado, um servidor
de hora, um relay de correio, uma base de dados, um broker MQTT, um endpoint
WebSocket. Cada serviço recebe uma barra de histórico, um tempo de resposta e uma
percentagem de disponibilidade na página do seu dispositivo.

Os últimos cinco vão além de abrir uma ligação: iniciam uma sessão real no
protocolo próprio do serviço. É essa a diferença entre «a porta 5432 está aberta»
e «a base de dados ainda aceita a minha conta e responde a uma consulta». O NTP
fica entre os dois: um único pedido e resposta, no protocolo próprio do NTP,
suficiente para ler o relógio do servidor sem uma sessão.

## Como funciona o estado de um serviço {#how-service-state-works}

Um serviço pode responder e estar mesmo assim em baixo: um `500`, uma palavra-chave
em falta, um certificado expirado. Por isso estes monitores escrevem um ponto em
cada verificação, `probe_success` valendo 1 ou 0, em vez de se calarem como um
dispositivo de hardware. É isso que torna possível uma percentagem de
disponibilidade: `avg_over_time(dumbmonit_probe_success[30d])`.

| Situação | O que acontece |
|---|---|
| Opção ou endereço inválido, sockets ICMP não permitidos (veja Ping) | Erro de configuração: nada é escrito, a página do dispositivo mostra o erro, sem alerta de «em baixo». |
| Ligação recusada, tempo limite, TLS recusado, `500`, palavra-chave ausente, perda total de pacotes, palavra-passe recusada, consulta falhada | `probe_success = 0`: a falha é registada, datada e contada. |
| Tudo bem | `probe_success = 1`. |

Cada monitor tem o seu próprio tempo limite (`timeout_seconds`, 5 s por
predefinição, 60 s no máximo), mais curto do que o `DUMBMONIT_PROBE_TIMEOUT_SECS`
do servidor: interrompido pelo agendador, não poderia escrever o seu zero.

Regras incorporadas que se aplicam a todos os onze: Service down (3 minutos),
Service flapping (mais de seis mudanças de estado em trinta minutos), Slow
service (mais de 3 s durante dez minutos). Para toda a verificação que lê um
certificado — `http`, `tls`, `smtp`, `mqtt`, `websocket`: Certificate expiring
soon (14 dias) e Certificate expired.

Uma palavra-passe recusada nunca é uma falha do serviço: é registada como
`reason="auth"`, para que o alerta diga o que corrigir.

### Métricas {#metrics}

Todos gauges, todos com o prefixo `dumbmonit_`, todos com a etiqueta
`probe="http|tcp|dns|ping|tls|smtp|postgres|mysql|mqtt|websocket|ntp"` além de
`target`, `host` e `tag_*`.

| Métrica | Monitores | Significado |
|---|---|---|
| `probe_success` | todos | 1 se o serviço responde corretamente, 0 caso contrário. |
| `probe_duration_seconds` | todos | Duração total da verificação. |
| `probe_failure_info` | todos | Presença (1) com uma etiqueta `reason`: `dns`, `connect`, `timeout`, `tls`, `cert_expired`, `status`, `keyword`, `json`, `body`, `packet_loss`, `record`, `auth`, `protocol`, `query`, `payload`. |
| `probe_http_status_code` | http, websocket | Código de estado obtido. |
| `probe_http_first_byte_seconds` | http | Tempo até aos cabeçalhos da resposta. |
| `probe_http_content_bytes` | http | Tamanho do corpo. |
| `probe_http_security_header{header}` | http | 0/1 por etiqueta `header` (apenas https): HSTS, nosniff, CSP, X-Frame-Options, frame-ancestors. |
| `probe_connect_seconds` | todos exceto dns e ping | Estabelecimento da ligação (início de sessão incluído para bases de dados). |
| `probe_tls_handshake_seconds` | tls, http, smtp, mqtt, websocket | Apenas a negociação TLS. |
| `probe_tls_version_info` | tls, http, smtp, mqtt, websocket | Presença; etiqueta `version`. |
| `probe_ssl_cert_expiry_days` | tls, http, smtp, mqtt, websocket | Dias até à expiração, negativo se expirado. |
| `probe_ssl_cert_valid` | tls, http, smtp, mqtt, websocket | 1 se a cadeia leva a uma autoridade conhecida. |
| `probe_ssl_cert_issuer_info` | tls, http, smtp, mqtt, websocket | Presença; etiqueta `issuer`. |
| `probe_dns_lookup_seconds` | dns | Tempo de resolução. |
| `probe_dns_answer_records` | dns | Registos do tipo pedido. |
| `probe_dns_answer_fingerprint` | dns | Com «Alertar quando a resposta muda»: uma impressão digital de 32 bits da resposta ordenada. O seu valor não significa nada; só as suas mudanças. |
| `probe_icmp_rtt_seconds`, `_min_seconds`, `_max_seconds` | ping | Média, mais curta e mais longa viagem de ida e volta. |
| `probe_icmp_packet_loss_ratio` | ping | Perda entre 0 e 1. |
| `probe_icmp_packets_sent`, `probe_icmp_packets_received` | ping | Echoes enviados e recebidos. |
| `probe_smtp_greeting_seconds` | smtp | Tempo até ao banner `220`. |
| `probe_smtp_ehlo_seconds` | smtp | Ida e volta do comando `EHLO`. |
| `probe_smtp_capabilities` | smtp | Extensões anunciadas. |
| `probe_smtp_authenticated` | smtp | 1 quando o `AUTH` teve êxito. |
| `probe_sql_query_seconds` | postgres, mysql | Apenas a execução da consulta. |
| `probe_sql_rows` | postgres, mysql | Linhas devolvidas. |
| `probe_sql_value` | postgres, mysql | Primeira coluna da primeira linha, quando é um número. |
| `probe_mqtt_connack_seconds` | mqtt | Tempo até ao `CONNACK`. |
| `probe_mqtt_suback_seconds` | mqtt | Tempo até ao `SUBACK`. |
| `probe_mqtt_message_bytes` | mqtt | Tamanho da mensagem retida. |
| `probe_mqtt_message_value` | mqtt | O seu conteúdo, quando é um número. |
| `probe_ws_handshake_seconds` | websocket | Handshake de abertura. |
| `probe_ntp_offset_seconds` | ntp | Desvio entre o relógio local e o servidor. |
| `probe_ntp_delay_seconds` | ntp | Atraso de ida e volta na rede. |
| `probe_ntp_stratum` | ntp | Distância a um relógio de referência (16: não sincronizado). |
| `probe_ntp_leap_indicator` | ntp | 0 a 3 (aviso de segundo intercalar, 3 = relógio não sincronizado). |
| `probe_ntp_root_delay_seconds` | ntp | Atraso acumulado até à raiz da hierarquia. |
| `probe_ntp_root_dispersion_seconds` | ntp | Dispersão acumulada até à raiz. |
| `probe_ntp_reference_info` | ntp | Presença; etiqueta `reference_id`. |
| `probe_ws_message_bytes` | websocket | Tamanho da primeira trama recebida. Só presente quando a verificação envia uma mensagem ou espera uma resposta (opções `send` / `expect`). |

## HTTP {#http}

**Site ou API web (HTTP)** — verifica que uma página ou uma API responde, com o
código de estado certo, o conteúdo certo, e um certificado válido. Exemplos: um
site, uma página de saúde de uma aplicação, uma API REST, a interface de um
serviço auto-alojado.

### Configuração {#setup}

1. No endereço, cole o URL completo da página a monitorizar. Sem «http://» nem
   «https://», assume-se HTTPS.
2. Prefira uma página leve que não exija início de sessão, por exemplo a página de
   saúde da aplicação («/health», «/status»), em vez da página inicial.
3. Se a página exige autenticação, preencha a credencial: um nome de utilizador e
   palavra-passe dão autenticação básica, um token é enviado como «Bearer».
4. Por predefinição, qualquer código de estado entre 200 e 299 serve. Para ir mais
   longe, exija uma palavra-chave na página ou um valor específico numa resposta
   JSON.
5. Em HTTPS, o certificado é lido automaticamente: será avisado antes de expirar.

!!! warning
    Nunca escreva uma palavra-passe ou um token nas opções: são copiados em texto
    simples em cada medição. Use o campo de credencial, que é cifrado.

Credenciais: nenhuma, nome de utilizador / palavra-passe (autenticação básica) ou
token de API (`Authorization: Bearer`).

### Opções {#options}

| Chave | Rótulo | Predefinição | Ajuda |
|---|---|---|---|
| `method` | Método HTTP | `GET` | GET serve para quase todos os casos. HEAD evita descarregar a página quando só o código de estado interessa. Escolhas: GET, HEAD, POST, PUT, PATCH, DELETE, OPTIONS. |
| `accepted_status` | Códigos de estado aceites | `200-299` | Códigos ou intervalos considerados normais, separados por vírgulas. Qualquer outro código conta como em baixo. Exemplo: `200-299,301,404`. |
| `keyword` | Palavra-chave esperada | *(vazio)* | Texto que tem de aparecer na página. Se faltar, o serviço é reportado em baixo mesmo que a página responda. |
| `keyword_absent` | A palavra-chave tem de estar ausente | `false` | Inverte a verificação: a presença da palavra-chave assinala uma falha. Útil para uma página de erro que responde 200. |
| `keyword_case_sensitive` | Respeitar maiúsculas na palavra-chave | `false` | Por predefinição, maiúsculas e minúsculas são tratadas da mesma forma. |
| `json_path` | Caminho JSON a verificar | *(vazio)* | Para uma resposta JSON: caminho do valor a verificar. Preencha-o juntamente com o valor esperado, nunca um sem o outro. Exemplo: `$.status`. |
| `json_expect` | Valor JSON esperado | *(vazio)* | Valor que o caminho JSON acima deve ter, por exemplo «ok» ou «true». |
| `headers` | Cabeçalhos extra | *(vazio)* | Um ou mais cabeçalhos «Nome: valor», separados por «\|». Não ponha segredos aqui: as opções são visíveis nos gráficos. |
| `body` | Corpo do pedido | *(vazio)* | Conteúdo enviado com o pedido, para métodos que o esperam (POST, PUT…). |
| `follow_redirects` | Seguir redirecionamentos | `true` | Desative para monitorizar o próprio redirecionamento, por exemplo um 301 para HTTPS. |
| `max_redirects` | Máximo de redirecionamentos seguidos | `10` | Entre 1 e 20. Sem efeito se os redirecionamentos não são seguidos. |
| `insecure_tls` | Aceitar um certificado não verificável | `false` | A verificação deixa de falhar com um certificado autoassinado ou emitido por uma autoridade privada. |
| `allow_private_targets` | Permitir alvos loopback e link-local | `false` | Por predefinição a verificação recusa endereços que só o próprio anfitrião do DumbMonit alcança (127.0.0.1, ::1, 169.254.x.x). Ative para monitorizar um serviço que corre no anfitrião do DumbMonit. Os endereços privados de LAN (10.x, 192.168.x) são sempre permitidos. |
| `check_certificate` | Ler o certificado | `true` | Em HTTPS, regista também a data de expiração do certificado para poder ser avisado antes de expirar. |
| `max_body_bytes` | Máximo de bytes lidos | `524288` | Além disto, o resto da página não é descarregado. A palavra-chave e o caminho JSON só são procurados nesta parte. |
| `user_agent` | Identidade anunciada (User-Agent) | `DumbMonit/<version>` | Nome que a verificação dá ao servidor. Altere-o se o servidor filtra robôs. |
| `timeout_seconds` | Tempo limite (segundos) | `5` | Tempo após o qual o serviço é reportado em baixo se não respondeu. Entre 1 e 60. |

## TCP {#tcp}

**Porta de rede (TCP)** — verifica que uma porta aceita ligações: SSH, base de
dados, partilha de ficheiros, servidor de jogos, impressora de rede.

### Configuração {#setup_1}

1. No endereço, escreva o anfitrião seguido da porta, separados por dois pontos:
   «nas.home.lan:22» ou «192.168.1.5:445».
2. Para um endereço IPv6, ponha-o entre parênteses retos: «[fd00::1]:445».
3. Pode também deixar o endereço sem porta e introduzi-la na opção «Porta»: uma
   das duas é obrigatória.
4. Só a abertura da ligação é testada: nenhum dado é enviado ao serviço, pelo que
   não tem efeito sobre ele.

!!! warning
    Uma porta aberta não prova que a aplicação por detrás funciona. Para um
    serviço web, prefira o tipo «Site ou API web», que lê a resposta.

### Opções {#options_1}

| Chave | Rótulo | Predefinição | Ajuda |
|---|---|---|---|
| `port` | Porta | *(vazio)* | Porta a abrir, se o endereço ainda não a indicar como «host:port». Uma das duas é obrigatória. |
| `allow_private_targets` | Permitir alvos loopback e link-local | `false` | Por predefinição a verificação recusa endereços que só o próprio anfitrião do DumbMonit alcança (127.0.0.1, ::1, 169.254.x.x). Ative para monitorizar um serviço que corre no anfitrião do DumbMonit. Os endereços privados de LAN (10.x, 192.168.x) são sempre permitidos. |
| `timeout_seconds` | Tempo limite (segundos) | `5` | Tempo após o qual o serviço é reportado em baixo se não respondeu. Entre 1 e 60. |

## DNS {#dns}

**Nome de domínio (DNS)** — verifica que um nome se resolve, e que aponta para o
endereço certo. Exemplos: o seu nome de domínio, um nome interno servido pelo seu
Pi-hole ou AdGuard, um registo MX.

### Configuração {#setup_2}

1. No endereço, escreva o nome a resolver, sem «http://»: «www.example.com».
2. Por predefinição, a verificação pede ao resolver do sistema um endereço IPv4
   (registo A). Escolha outro tipo de registo nas opções se necessário.
3. Para monitorizar o seu próprio servidor DNS, introduza o seu endereço IP em
   «Servidor DNS a consultar»: a verificação falhará se ele deixar de responder.
4. Para detetar sequestro ou uma configuração errada, liste em «Valores esperados»
   os endereços que a resposta deve conter.

!!! warning
    O servidor DNS é indicado pelo seu endereço IP, não por um nome: seria preciso
    um resolver para resolver o resolver.

### Opções {#options_2}

| Chave | Rótulo | Predefinição | Ajuda |
|---|---|---|---|
| `record_type` | Tipo de registo | `A` | A para um endereço IPv4, AAAA para IPv6, MX para correio, CNAME para um alias… Escolhas: A, AAAA, CNAME, MX, TXT, NS, SOA, SRV, PTR, CAA. |
| `resolver` | Servidor DNS a consultar | *(vazio)* | Endereço IP de um resolver, porta opcional. Vazio: o resolver do sistema. Útil para verificar o seu próprio servidor DNS. Exemplo: `1.1.1.1` ou `10.0.0.1:5353`. |
| `expect` | Valores esperados | *(vazio)* | Valores que têm de aparecer todos na resposta, separados por vírgulas. Vazio: só a resolução é verificada. |
| `expect_mode` | Comparação | `contains` | `contains`: cada valor esperado tem de aparecer algures na resposta. `exact`: a resposta tem de conter esses valores e nada mais. |
| `forbid` | Valores proibidos | *(vazio)* | Valores que nunca devem aparecer na resposta, separados por vírgulas. Verificados antes dos esperados. |
| `alert_on_change` | Alertar quando a resposta muda | `false` | Regista uma impressão digital da resposta ordenada e alerta sempre que muda, sem ser preciso escrever os valores esperados. |
| `timeout_seconds` | Tempo limite (segundos) | `5` | Tempo após o qual o serviço é reportado em baixo se não respondeu. Entre 1 e 60. |

### Afirmar o que uma zona responde {#asserting-what-a-zone-answers}

`expect` sozinho apanha o registo que desapareceu. Não apanha o registo que foi
*acrescentado*: uma zona sequestrada mantém muitas vezes o `A` legítimo e põe um
segundo ao lado, e um cliente chega então a qualquer um deles ao acaso.

- `expect_mode = exact` recusa qualquer valor que a expectativa não cubra. A
  ordem não importa — o DNS não mantém uma.
- `forbid` nomeia valores que nunca devem voltar: o endereço de um anfitrião
  antigo, um `TXT` de validação expirado, um servidor de correio que
  desativou. É verificado primeiro, pelo que prevalece sobre `expect`.

Ambos comparam sem distinguir maiúsculas, ignoram o ponto final de um nome e as
aspas à volta de um `TXT`, e tratam um valor esperado como encontrado quando
aparece dentro de um registo — pelo que um `MX` corresponde a `mail.example.com`
sem ter de copiar a sua prioridade.

### Alertar quando a resposta muda {#alerting-when-the-answer-changes}

Escrever os valores esperados nem sempre é prático: um nome servido por uma CDN,
registos que outra pessoa gere, ou simplesmente uma zona sobre a qual quer ser
avisado sempre que se mexe. Marque **Alertar quando a resposta muda**: cada
verificação regista então uma impressão digital da resposta, e a regra
incorporada [DNS answer changed](../alerting/rules.md#reverse-proxies-and-domains)
dispara quando difere da anterior. O alerta resolve-se sozinho uma hora depois; a
nova resposta passa a ser a referência a partir daí.

A impressão digital cobre os valores da resposta, ordenados e sem duplicados, com
a cadeia CNAME incluída: um round-robin que devolve os mesmos endereços por outra
ordem não a altera, nem um TTL. Um nome cuja resposta muda em cada consulta (um
pool grande como `pool.ntp.org`, ou DNS geográfico que responde diferente de um
resolver para o seguinte) alertaria o tempo todo: deixe a opção desativada para
esses, ou consulte um resolver fixo.

## Ping {#ping}

**Anfitrião alcançável (ping)** — envia alguns echoes ICMP e mede o tempo de
resposta e a perda de pacotes. Exemplos: router ou gateway doméstico, ponto de
acesso Wi-Fi, impressora, máquina sem agente nem SNMP, anfitrião remoto.

### Configuração {#setup_3}

1. No endereço, escreva o nome de anfitrião ou endereço IP: «192.168.1.1» ou
   «router.home.lan».
2. Em cada sondagem, a verificação envia quatro echoes e mede o tempo de resposta,
   juntamente com a parte de pacotes perdidos.
3. Por predefinição, só uma perda total conta como em baixo; a perda parcial
   continua visível nos gráficos. Baixe «Perda tolerada» para ser avisado mais
   cedo.
4. Se o DumbMonit corre no Docker, mantenha as linhas `sysctls:` do
   docker-compose.yml fornecido sob o serviço dumbmonit: permitem ao utilizador
   sem privilégios do contentor abrir sockets de echo ICMP. Não é necessária
   nenhuma capability.

!!! warning
    Sem esse sysctl, a verificação não consegue abrir um socket ICMP: reporta-o
    como erro de configuração, não como falha do anfitrião. Alguns dispositivos
    também ignoram pings de propósito: verifique isso antes de tirar conclusões.

```yaml
services:
  dumbmonit:
    sysctls:
      net.ipv4.ping_group_range: "0 2147483647"
```

Fora do Docker, `sysctl -w net.ipv4.ping_group_range="0 2147483647"` no
anfitrião faz o mesmo, e um socket raw (`NET_RAW`, root) também funciona.

### Opções {#options_3}

| Chave | Rótulo | Predefinição | Ajuda |
|---|---|---|---|
| `count` | Número de echoes | `4` | Pacotes enviados em cada sondagem, de 1 a 20. |
| `packet_timeout_ms` | Espera por pacote (ms) | `1000` | Tempo permitido para cada resposta, de 50 a 10 000 ms. |
| `interval_ms` | Intervalo entre dois echoes (ms) | `100` | Pausa entre dois echoes, de 0 a 5 000 ms. |
| `payload_bytes` | Tamanho da carga (bytes) | `56` | Dados transportados por cada echo, de 0 a 1 400 bytes. |
| `ip_version` | Versão IP | `auto` | «auto» toma o primeiro endereço resolvido; force 4 ou 6 se o anfitrião tem ambos e um não responde. Escolhas: auto, 4, 6. |
| `loss_threshold_percent` | Perda tolerada (%) | `100` | Acima desta percentagem de pacotes perdidos, o anfitrião é reportado em baixo. 100: só uma perda total conta, a perda parcial continua visível nos gráficos. |
| `timeout_seconds` | Tempo limite (segundos) | `5` | Orçamento total da sondagem, de 1 a 60. Recusado se não cobrir «número de echoes × espera por pacote». |

## TLS {#tls}

**Certificado TLS** — verifica que um certificado é válido e avisa antes de
expirar, em qualquer porta cifrada. Exemplos: servidor de correio (IMAPS,
SMTPS), proxy inverso, diretório LDAPS, broker MQTT, interface de administração.

### Configuração {#setup_4}

1. No endereço, escreva o anfitrião e, se não for a 443, a porta do serviço
   cifrado: «mail.example.com:993», «ldap.home.lan:636».
2. A verificação abre uma ligação cifrada, lê o certificado apresentado e fecha
   logo: nenhum dado é trocado com a aplicação.
3. Regista o número de dias até à expiração, o emissor e a versão TLS. As regras
   de alerta incluídas avisam aos catorze dias, depois na expiração.
4. Se o serviço é alcançado pelo seu endereço IP mas o certificado traz um nome,
   introduza esse nome em «Nome do servidor (SNI)».
5. Para um certificado emitido pela sua própria autoridade, marque «Aceitar um
   certificado não verificável»: a data de expiração continua monitorizada.

!!! warning
    Para um site, o tipo «Site ou API web» já lê o certificado: este tipo destina-se
    a serviços que não falam HTTP, ou cuja aplicação não quer atingir.

O handshake completa-se mesmo quando o certificado é recusado: é isso que permite
ler a data de expiração de um certificado que já expirou. O veredicto da
verificação está em `probe_ssl_cert_valid`.

### Opções {#options_4}

| Chave | Rótulo | Predefinição | Ajuda |
|---|---|---|---|
| `server_name` | Nome do servidor (SNI) | *(vazio)* | Nome de domínio anunciado ao servidor e verificado no certificado. Defina-o quando o endereço é um IP atrás de um proxy inverso. |
| `insecure_tls` | Aceitar um certificado não verificável | `false` | Uma cadeia não verificável deixa de contar como falha: a data de expiração continua registada. |
| `allow_private_targets` | Permitir alvos loopback e link-local | `false` | Por predefinição a verificação recusa endereços que só o próprio anfitrião do DumbMonit alcança (127.0.0.1, ::1, 169.254.x.x). Ative para monitorizar um serviço que corre no anfitrião do DumbMonit. Os endereços privados de LAN (10.x, 192.168.x) são sempre permitidos. |
| `timeout_seconds` | Tempo limite (segundos) | `5` | Tempo após o qual o serviço é reportado em baixo se não respondeu. Entre 1 e 60. |

## SMTP {#smtp}

**Relay de correio (SMTP)** — abre uma sessão real num servidor de correio:
saudação, EHLO, STARTTLS, e o início de sessão se indicar um. Exemplos: o relay
SMTP do seu fornecedor, um Postfix ou msmtp local, o Proxmox Mail Gateway, a
porta de submissão de um servidor de correio.

### Configuração {#setup_5}

1. No endereço, escreva o servidor de correio, e a porta se não for a habitual:
   «smtp.example.com» ou «smtp.example.com:2525».
2. Escolha a cifragem que o seu relay usa: STARTTLS para a porta de submissão 587,
   TLS para a porta 465, None para um relay na sua própria rede a escutar na
   porta 25.
3. Para verificar que o relay ainda aceita a sua conta, preencha a credencial: a
   verificação executa então AUTH e reporta uma palavra-passe recusada como tal,
   não como uma falha.
4. Nada é alguma vez enviado: a verificação pára depois da saudação, das extensões
   e do início de sessão opcional, depois desliga. Nenhuma mensagem entra na fila.
5. Para ser avisado quando uma extensão desaparece, nomeie-a em «Extensão
   esperada»: um relay que deixa de anunciar STARTTLS é um relay que enviaria o
   seu correio em texto simples.

!!! warning
    Uma sessão aceite não prova que o correio sai. Um relay que aceita tudo e o
    põe em fila para sempre responde perfeitamente aqui: vigie as próprias filas
    para isso.

Credenciais: nenhuma, ou um nome de utilizador e palavra-passe enviados com
`AUTH PLAIN` (ou `AUTH LOGIN` quando o servidor oferece apenas isso). São enviados
**depois** de a ligação ser cifrada e de o certificado ter sido julgado: um relay
cuja cadeia não pode ser verificada nunca os recebe.

Em TLS ou STARTTLS, o certificado é lido como a verificação TLS o lê — expiração,
emissor, versão do protocolo — pelo que um relay de correio é também um
certificado sobre o qual é avisado.

### O que não consegue detetar {#what-it-cannot-detect}

Que o correio realmente sai. Um relay que aceita todas as sessões e depois adia
tudo responde perfeitamente aqui; vigie antes as suas filas (veja a página do
Proxmox Mail Gateway). Também nunca envia uma mensagem, pelo que nada diz sobre
filtragem de destinatários, SPF ou DKIM.

### Opções {#options_5}

| Chave | Rótulo | Predefinição | Ajuda |
|---|---|---|---|
| `security` | Cifragem | `starttls` | STARTTLS começa em texto simples e atualiza (porta 587), TLS cifra desde o primeiro byte (porta 465), None nunca cifra (porta 25) e só serve para um relay na sua própria rede. |
| `port` | Porta | *(vazio)* | Usada se o endereço não indicar uma. Vazio: 587 com STARTTLS, 465 com TLS, 25 sem cifragem. |
| `helo_name` | Nome anunciado (EHLO) | `dumbmonit` | Nome que a verificação dá quando se apresenta. Um relay estrito recusa um nome que não consegue resolver. |
| `expect_capability` | Extensão esperada | *(vazio)* | Extensão que o servidor tem de anunciar na sua resposta EHLO, por exemplo STARTTLS ou AUTH. Vazio: sem expectativa. |
| `server_name` | Nome do servidor (SNI) | *(o anfitrião)* | Nome de domínio anunciado ao servidor e verificado no certificado. Defina-o quando o endereço é um IP. |
| `insecure_tls` | Aceitar um certificado não verificável | `false` | Uma cadeia não verificável deixa de contar como falha: a data de expiração continua registada. |
| `allow_private_targets` | Permitir alvos loopback e link-local | `false` | Veja «Endereços que as verificações recusam» abaixo. |
| `timeout_seconds` | Tempo limite (segundos) | `5` | Tempo após o qual o serviço é reportado em baixo se não respondeu. Entre 1 e 60. |

### Motivos de falha {#failure-reasons}

| `reason` | O que aconteceu |
|---|---|
| `connect` | A porta está fechada ou o relay é inacessível. |
| `protocol` | Algo respondeu, mas não SMTP — ou recusou a ligação, o `EHLO`, ou o STARTTLS que tinha anunciado. |
| `auth` | A conta foi recusada (`535`, `530`, `534`). O relay está bem; a credencial não. |
| `payload` | A extensão nomeada em «Extensão esperada» não é anunciada. |
| `tls`, `cert_expired` | Mesmo significado que na verificação TLS. |

## PostgreSQL {#postgresql}

**Base de dados PostgreSQL** — liga-se, autentica-se e executa uma consulta: o
sinal de uma base de dados que está de pé mas já não responde. Exemplos: a base
de dados por detrás do Nextcloud ou do Immich, um recorder do Home Assistant, uma
base de dados de aplicação, uma instância TimescaleDB.

### Configuração {#setup_6}

1. No servidor, crie uma conta para monitorização e não lhe dê mais do que o
   direito de se ligar: CREATE ROLE dumbmonit LOGIN PASSWORD 'a-long-password';
2. Garanta que essa conta pode alcançar o servidor a partir da máquina do
   DumbMonit, o que normalmente significa mais uma linha em pg_hba.conf seguida de
   um recarregamento da configuração.
3. No endereço, escreva o servidor, e a porta se não for a 5432: «db.home.lan» ou
   «db.home.lan:5433».
4. A consulta predefinida é SELECT 1, que não lê nenhuma tabela: nada mais precisa
   de ser concedido. O tempo de ligação e o tempo da consulta são registados
   separadamente.
5. Para vigiar algo seu, substitua a consulta por uma que devolva um único número,
   e torna-se um gráfico: um comprimento de fila, uma contagem de linhas, um
   atraso de replicação.

!!! warning
    A verificação abre uma ligação real em cada sondagem. Numa instância já perto
    do seu limite de ligações, dê-lhe um intervalo maior.

Credenciais: um nome de utilizador e palavra-passe, obrigatórios. Uma base de
dados nunca é monitorizada anonimamente, pelo que o formulário não oferece mais
nada.

### O que não consegue detetar {#what-it-cannot-detect_1}

Atraso de replicação, um bloqueio que se arrasta, uma tabela que não para de
crescer. Tudo isso vive nas vistas de administração do próprio motor — escreva a
consulta você próprio, e o número que ela devolve torna-se `probe_sql_value`, um
gráfico sobre o qual pode alertar.

### Opções {#options_6}

| Chave | Rótulo | Predefinição | Ajuda |
|---|---|---|---|
| `port` | Porta | `5432` | Usada se o endereço não indicar uma. |
| `database` | Base de dados | `postgres` | Base de dados aberta na ligação. A conta de monitorização tem de poder ligar-se a ela. |
| `query` | Consulta | `SELECT 1` | Executada em cada verificação. A predefinida não lê nenhuma tabela, pelo que a conta não precisa de privilégio além de se ligar. Uma consulta que devolva um número transforma-o num gráfico. |
| `expect` | Valor esperado | *(vazio)* | Valor que a primeira coluna da primeira linha tem de ter. Vazio: só se verifica que a consulta tem êxito. |
| `sslmode` | Cifragem | `prefer` | `prefer` cifra quando o servidor o oferece, `require` recusa ligar sem isso, `verify-full` verifica também o certificado e o nome, `disable` nunca cifra. |
| `allow_private_targets` | Permitir alvos loopback e link-local | `false` | Veja «Endereços que as verificações recusam» abaixo. |
| `timeout_seconds` | Tempo limite (segundos) | `5` | Tempo após o qual o serviço é reportado em baixo se não respondeu. Entre 1 e 60. |

## MySQL e MariaDB {#mysql-and-mariadb}

**Base de dados MySQL ou MariaDB** — liga-se, autentica-se e executa uma
consulta: o sinal de uma base de dados que está de pé mas já não responde.
Exemplos: a base de dados por detrás de um WordPress, de um Nextcloud ou de um
Kimai, uma base de dados de aplicação, um MariaDB no Docker.

### Configuração {#setup_7}

1. No servidor, crie uma conta para monitorização sem qualquer privilégio: CREATE
   USER 'dumbmonit'@'%' IDENTIFIED BY 'a-long-password';
2. Verifique que a conta pode ligar-se a partir da máquina do DumbMonit: um padrão
   de anfitrião localhost só aceitaria ligações feitas no próprio servidor.
3. No endereço, escreva o servidor, e a porta se não for a 3306: «db.home.lan» ou
   «db.home.lan:3307».
4. A consulta predefinida é SELECT 1, que não lê nenhuma tabela: nada mais precisa
   de ser concedido. O tempo de ligação e o tempo da consulta são registados
   separadamente.
5. Para vigiar algo seu, substitua a consulta por uma que devolva um único número,
   e torna-se um gráfico: um comprimento de fila, uma contagem de linhas, um
   atraso de replicação.

!!! warning
    A verificação abre uma ligação real em cada sondagem. Numa instância já perto
    do seu limite de ligações, dê-lhe um intervalo maior.

### Opções {#options_7}

| Chave | Rótulo | Predefinição | Ajuda |
|---|---|---|---|
| `port` | Porta | `3306` | Usada se o endereço não indicar uma. |
| `database` | Base de dados | *(vazio)* | Base de dados aberta na ligação. Vazio: nenhuma, o que basta para a consulta predefinida. |
| `query` | Consulta | `SELECT 1` | Executada em cada verificação. A predefinida não lê nenhuma tabela, pelo que a conta não precisa de privilégio além de se ligar. Uma consulta que devolva um número transforma-o num gráfico. |
| `expect` | Valor esperado | *(vazio)* | Valor que a primeira coluna da primeira linha tem de ter. Vazio: só se verifica que a consulta tem êxito. |
| `sslmode` | Cifragem | `prefer` | `prefer` cifra quando o servidor o oferece, `require` recusa ligar sem isso, `verify-full` verifica também o certificado e o nome, `disable` nunca cifra. |
| `allow_private_targets` | Permitir alvos loopback e link-local | `false` | Veja «Endereços que as verificações recusam» abaixo. |
| `timeout_seconds` | Tempo limite (segundos) | `5` | Tempo após o qual o serviço é reportado em baixo se não respondeu. Entre 1 e 60. |

### Motivos de falha, ambos os motores {#failure-reasons-both-engines}

| `reason` | O que aconteceu |
|---|---|
| `connect` | A porta está fechada, o servidor é inacessível, ou recusou a ligação por um motivo que não a conta (a arrancar, demasiadas ligações). |
| `auth` | A conta foi recusada: palavra-passe errada, sem linha em `pg_hba.conf`, padrão de anfitrião que não cobre a máquina do DumbMonit. |
| `query` | Ligado e autenticado, mas a consulta falhou: tabela em falta, base de dados só de leitura, erro de sintaxe. |
| `payload` | A consulta funcionou mas devolveu algo diferente do valor esperado. |
| `timeout` | A ligação ou a consulta não terminou a tempo — o caso «de pé mas lento». |

## MQTT {#mqtt}

**Broker MQTT** — liga-se ao broker, subscreve um tópico, e pode esperar por uma
mensagem retida. Exemplos: Mosquitto, o broker por detrás do Home Assistant,
Zigbee2MQTT, sensores ESPHome.

### Configuração {#setup_8}

1. No endereço, escreva o broker, e a porta se não for a habitual:
   «broker.home.lan» ou «broker.home.lan:1884».
2. Marque «Ligação cifrada» para um broker a escutar na 8883, e preencha a
   credencial se exigir uma conta.
3. Ligar já vale a pena monitorizar: um broker que recusa a sua conta, ou que
   deixou de responder, é a razão pela qual a casa inteira ficou muda.
4. Para ir mais longe, nomeie um tópico: a verificação subscreve-o então, e um
   broker que recusa a subscrição diz-lhe que a conta perdeu os direitos de
   acesso.
5. Marque «Esperar uma mensagem retida» apenas para um tópico que a transporte,
   tipicamente um tópico de disponibilidade. Um tópico em que se publica de vez
   em quando parece silencioso a um cliente que acabou de se ligar.

!!! warning
    A verificação nunca publica nada. Também não consegue saber quão antiga é uma
    mensagem retida: o MQTT não as data, pelo que um valor congelado há três
    semanas continua a contar como presente.

### O que não consegue detetar {#what-it-cannot-detect_2}

Que um dado sensor deixou de emitir. A mensagem retida que a verificação lê é
tudo o que o broker guardou, possivelmente com três semanas — o MQTT não a
carimba no tempo. Para frescura, vigie a automação que a consome, ou use um
heartbeat.

### Opções {#options_8}

| Chave | Rótulo | Predefinição | Ajuda |
|---|---|---|---|
| `tls` | Ligação cifrada | `false` | Cifra a sessão desde o primeiro byte, como os brokers fazem na porta 8883. |
| `port` | Porta | *(vazio)* | Usada se o endereço não indicar uma. Vazio: 1883 em texto simples, 8883 cifrada. |
| `topic` | Tópico | *(vazio)* | Tópico que a verificação subscreve. Vazio: apenas se liga, o que já lhe diz que o broker está vivo e aceita a sua conta. |
| `expect_message` | Esperar uma mensagem retida | `false` | Uma mensagem tem de chegar nesse tópico antes do tempo limite. Só as mensagens retidas chegam logo: um tópico em que apenas se publica de vez em quando parecerá silencioso. |
| `expect` | Conteúdo esperado | *(vazio)* | Texto que essa mensagem tem de conter. Preenchê-lo implica esperar uma mensagem. |
| `client_id` | Identificador do cliente | *(derivado)* | Nome anunciado ao broker. A predefinição deriva do dispositivo, para que duas verificações nunca se desliguem uma à outra. |
| `server_name` | Nome do servidor (SNI) | *(o anfitrião)* | Nome de domínio anunciado ao broker e verificado no certificado. Defina-o quando o endereço é um IP. |
| `insecure_tls` | Aceitar um certificado não verificável | `false` | Uma cadeia não verificável deixa de contar como falha: a data de expiração continua registada. |
| `allow_private_targets` | Permitir alvos loopback e link-local | `false` | Veja «Endereços que as verificações recusam» abaixo. |
| `timeout_seconds` | Tempo limite (segundos) | `5` | Tempo após o qual o serviço é reportado em baixo se não respondeu. Entre 1 e 60. |

### Motivos de falha {#failure-reasons_1}

| `reason` | O que aconteceu |
|---|---|
| `connect` | A porta está fechada ou o broker é inacessível. |
| `auth` | O `CONNACK` recusou a conta, ou o broker recusou a subscrição — uma lista de controlo de acesso já não cobre este tópico. |
| `protocol` | Algo respondeu mas não fala MQTT 3.1.1, ou o broker rejeitou o identificador do cliente. |
| `payload` | Nenhuma mensagem retida chegou no tópico, ou o seu conteúdo não contém o texto esperado. |

## WebSocket {#websocket}

**Endpoint WebSocket** — executa o handshake de upgrade, e pode enviar uma trama
e esperar por uma: uma falha diferente de uma página HTTP simples. Exemplos: a
API do Home Assistant, um dashboard em direto, um stream de registos, um endpoint
atrás de um proxy inverso.

### Configuração {#setup_9}

1. No endereço, cole o endpoint completo: «wss://home.example.com/api/websocket».
   Sem esquema, assume-se wss.
2. A verificação executa todo o handshake de abertura e verifica a resposta que o
   servidor calcula a partir da chave que lhe foi enviada: um proxy inverso que
   perdeu o cabeçalho Upgrade é apanhado aqui, onde uma verificação HTTP ainda
   veria um 200 saudável.
3. Mais nada é preciso para a maioria dos endpoints. Um estado diferente de 101 é
   reportado com o seu código, pelo que um 401 de um token expirado não é
   confundido com uma falha.
4. Para ir mais longe, preencha «Conteúdo esperado» com um fragmento da primeira
   trama que o servidor envia por si, por exemplo a saudação da API do Home
   Assistant.
5. Se o endpoint não diz nada até lhe falarem, ponha uma mensagem em «Trama a
   enviar» e o que a sua resposta deve conter em «Conteúdo esperado».

!!! warning
    A verificação abre e desliga em menos de um segundo. Uma ligação que uma
    firewall ou o tempo limite de um proxy corta ao fim de trinta segundos parece
    perfeitamente saudável aqui.

Credenciais: nenhuma, um nome de utilizador e palavra-passe (enviados como
autenticação HTTP básica no pedido de abertura) ou um token (enviado como
`Authorization: Bearer`).

### Opções {#options_9}

| Chave | Rótulo | Predefinição | Ajuda |
|---|---|---|---|
| `path` | Caminho | *(do endereço)* | Caminho do pedido de abertura. Tomado do endereço quando este traz um. |
| `port` | Porta | *(vazio)* | Usada se o endereço não indicar uma. Vazio: 443 para wss, 80 para ws. |
| `send` | Trama a enviar | *(vazio)* | Trama de texto enviada quando a ligação está aberta. Vazio: nada é enviado. |
| `expect` | Conteúdo esperado | *(vazio)* | Texto que uma trama recebida tem de conter. Vazio e sem nada a enviar, só o handshake de abertura é verificado. |
| `subprotocol` | Subprotocolo | *(vazio)* | Valor de Sec-WebSocket-Protocol. A verificação falha se o servidor escolher outro. |
| `origin` | Origin | *(vazio)* | Valor do cabeçalho Origin, que alguns servidores exigem antes de fazer upgrade. |
| `server_name` | Nome do servidor (SNI) | *(o anfitrião)* | Nome de domínio anunciado ao servidor e verificado no certificado. Defina-o quando o endereço é um IP. |
| `insecure_tls` | Aceitar um certificado não verificável | `false` | Uma cadeia não verificável deixa de contar como falha: a data de expiração continua registada. |
| `allow_private_targets` | Permitir alvos loopback e link-local | `false` | Veja «Endereços que as verificações recusam» abaixo. |
| `timeout_seconds` | Tempo limite (segundos) | `5` | Tempo após o qual o serviço é reportado em baixo se não respondeu. Entre 1 e 60. |

### Motivos de falha {#failure-reasons_2}

| `reason` | O que aconteceu |
|---|---|
| `connect` | A porta está fechada ou o anfitrião é inacessível. |
| `status` | O endpoint respondeu algo diferente de `101`; `probe_http_status_code` traz o código. |
| `protocol` | Respondeu `101` mas o seu `Sec-WebSocket-Accept` não corresponde à chave enviada, escolheu outro subprotocolo, ou fechou logo a ligação. |
| `payload` | Nenhuma trama chegou, ou não contém o texto esperado. |

## NTP {#ntp}

**Servidor de hora NTP** — envia um único pedido SNTP e lê a resposta: desvio,
atraso de ida e volta, stratum e indicador de segundo intercalar. Exemplos: um
router ou NAS a atuar como servidor de hora local, um servidor NTP interno
(chrony, ntpd), um servidor de pool público usado como referência.

### Configuração {#setup_10}

1. No endereço, escreva o nome ou endereço IP do servidor: «pool.ntp.org» ou
   «192.168.1.1». A porta UDP 123 é usada a menos que a opção «Porta» diga outra
   coisa.
2. A verificação envia um único pedido SNTP e lê a resposta: nenhum estado é
   mantido entre sondagens, e nada é escrito no servidor.
3. Regista o desvio entre o relógio do servidor e o anfitrião do DumbMonit, o
   atraso de ida e volta, o stratum e o indicador de segundo intercalar.
4. Um servidor que responde stratum 16, ou cujo indicador de segundo intercalar
   lê «unsynchronized», nunca conseguiu sincronizar: a verificação reporta-o em
   baixo seja qual for o desvio.
5. Baixe «Desvio tolerado» para ser avisado mais cedo do que 100 ms; aumente-o para
   um servidor a vários saltos de um relógio de referência.

!!! warning
    O desvio é tão bom quanto o relógio do próprio anfitrião do DumbMonit: se esse
    anfitrião não for ele próprio mantido sincronizado (NTP, sincronização de hora
    de um hipervisor), o desvio medido reflete tanto a sua deriva como a do
    servidor.

Credenciais: nenhuma.

### Opções {#options_10}

| Chave | Rótulo | Predefinição | Ajuda |
|---|---|---|---|
| `port` | Porta | `123` | Porta UDP do servidor NTP. |
| `offset_threshold_ms` | Desvio tolerado (ms) | `100` | Acima deste desvio de relógio, o servidor é reportado em baixo. Um servidor não sincronizado (stratum 16) falha sempre, seja qual for este valor. |
| `allow_private_targets` | Permitir alvos loopback e link-local | `false` | Veja «Endereços que as verificações recusam» abaixo. |
| `timeout_seconds` | Tempo limite (segundos) | `5` | Tempo após o qual o serviço é reportado em baixo se não respondeu. Entre 1 e 60. |

### Motivos de falha {#failure-reasons_3}

| `reason` | O que aconteceu |
|---|---|
| `connect` | O socket UDP não pôde ser aberto, ou a resposta veio de um endereço inesperado. |
| `timeout` | Nenhuma resposta chegou dentro do tempo limite. |
| `protocol` | A resposta não era um pacote NTP válido, ou não veio de um servidor (modo errado). |
| `unsynchronized` | O servidor respondeu stratum 16 ou 0 (um «kiss-o'-death», que também nomeia o motivo no registo), ou o seu indicador de segundo intercalar lê «unsynchronized». |
| `clock_offset` | O desvio medido excede o «Desvio tolerado». |

## Endereços que as verificações recusam {#addresses-the-checks-refuse}

As verificações que se ligam a um endereço que lhes dá — HTTP, TCP, TLS, NTP,
SMTP, PostgreSQL, MySQL, MQTT, WebSocket — reportam o que veem — o que é também,
palavra por palavra, o que faz uma falsificação de pedidos do lado do servidor
(SSRF). Para impedir um administrador de monitorização de ler serviços que só o
anfitrião do DumbMonit alcança (a base de dados de métricas incorporada em
`127.0.0.1:8428`, o serviço de metadados de um fornecedor de nuvem em
`169.254.169.254`), as verificações recusam por predefinição:

- endereços loopback (`127.0.0.0/8`, `::1`);
- endereços link-local (`169.254.0.0/16`, `fe80::/10`), incluindo o endereço de
  metadados da nuvem, e o endereço não especificado (`0.0.0.0`, `::`).

Os intervalos de LAN privada (`10.0.0.0/8`, `172.16.0.0/12`, `192.168.0.0/16`,
`fc00::/7`) são sempre permitidos: monitorizá-los é o objetivo de um homelab.

A regra aplica-se ao endereço que escreveu, a todos os endereços para que um nome
de anfitrião resolve (verificados de novo no momento da ligação, para que um nome
que mude a sua resposta não passe), e a cada redirecionamento que a verificação
HTTP segue. Um endereço recusado aparece como erro de configuração no alvo —
nenhum alerta de «em baixo» é enviado — nomeando a opção a ativar:
`allow_private_targets`. Ative-a para um alvo que realmente monitoriza um serviço
no próprio anfitrião do DumbMonit.

Um redirecionamento para um endereço recusado é reportado como medição falhada
(`reason="connect"`), porque é o serviço monitorizado que mudou, não a
configuração.

Os valores encontrados num `json_path` nunca são repetidos além de 32 caracteres
numa mensagem de falha.

## Erros comuns {#common-errors}

| Sintoma | Causa provável |
|---|---|
| O ping mostra um erro de configuração | Falta o sysctl `net.ipv4.ping_group_range` no ficheiro Compose. Veja acima. |
| HTTP em baixo com `reason="keyword"` | A palavra-chave só é procurada nos primeiros `max_body_bytes`, sem distinguir maiúsculas a menos que `keyword_case_sensitive` esteja definido. |
| HTTP em baixo com `reason="tls"` | Autoassinado ou autoridade privada: defina `insecure_tls`. |
| Uma verificação mostra um erro de configuração a nomear `allow_private_targets` | O endereço é loopback ou link-local. Veja «Endereços que as verificações recusam» acima. |
| TCP em baixo embora `telnet host port` funcione a partir do seu portátil | O contentor do DumbMonit não consegue alcançar o anfitrião (rede Docker, firewall). Teste a partir da rede do contentor. |
| DNS em baixo com `reason="record"` | A resposta não contém todos os valores listados em `expect`. |
| Ping em baixo com `reason="packet_loss"` | Perda acima de `loss_threshold_percent`; alguns dispositivos limitam o ICMP. |
| DNS em baixo com `reason="record"` embora `expect` corresponda | `expect_mode` é `exact` e a resposta contém um valor extra, ou `forbid` correspondeu. |
| SMTP em baixo com `reason="protocol"` em `security = starttls` | O relay não anuncia STARTTLS nesta porta. Use `tls` para a 465, `none` para um relay local simples. |
| Uma verificação de base de dados em baixo com `reason="auth"` | A conta existe mas não pode ligar-se a partir da máquina do DumbMonit: verifique `pg_hba.conf`, ou a parte de anfitrião do utilizador MySQL. |
| Uma verificação de base de dados em baixo com `reason="query"` | A ligação e o início de sessão funcionaram. A base de dados está de pé e recusa trabalhar: só de leitura, tabela em falta, recuperação. |
| MQTT em baixo com `reason="payload"` num tópico que claramente tem tráfego | Só as mensagens *retidas* chegam a um cliente que acabou de subscrever. Publique esse tópico com a flag retain, ou desmarque «Esperar uma mensagem retida». |
| WebSocket em baixo com `reason="protocol"` após um `101` | Algo no caminho responde ao upgrade sem falar WebSocket — normalmente uma regra de proxy inverso que engole o cabeçalho `Upgrade`. |
