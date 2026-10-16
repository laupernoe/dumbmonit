# Serviços: HTTP, TCP, DNS, ping, TLS, NTP, SMTP, SQL, MQTT, WebSocket {#services-http-tcp-dns-ping-tls-ntp-smtp-sql-mqtt-websocket}

Onze monitores observam **serviços** em vez de equipamentos, estilo
Uptime Kuma: uma página web, uma porta, um nome DNS, um host, um
certificado, um servidor de hora, um relay de e-mail, um banco de
dados, um broker MQTT, um endpoint WebSocket. Cada serviço recebe uma
barra de histórico, um tempo de resposta e uma porcentagem de
disponibilidade em sua página de monitor.

Os últimos cinco vão além de abrir uma conexão: começam uma sessão real
no próprio protocolo do serviço. Essa é a diferença entre "a porta 5432
está aberta" e "o banco de dados ainda aceita minha conta e responde
uma consulta". O NTP fica entre os dois: uma única requisição e
resposta, no próprio protocolo NTP, suficiente para ler o relógio do
servidor sem uma sessão.

## Como o estado do serviço funciona {#how-service-state-works}

Um serviço pode responder e ainda estar fora do ar: um `500`, uma
palavra-chave faltando, um certificado expirado. Então esses monitores
escrevem um ponto em toda verificação, com `probe_success` sendo 1 ou
0, em vez de ficar silencioso como um dispositivo de hardware. É isso
que torna possível uma porcentagem de disponibilidade:
`avg_over_time(dumbmonit_probe_success[30d])`.

| Situação | O que acontece |
|---|---|
| Opção ou endereço inválido, sockets ICMP não permitidos (veja Ping) | Erro de configuração: nada é escrito, a página do monitor mostra o erro, nenhum alerta de "fora do ar". |
| Conexão recusada, tempo esgotado, TLS recusado, `500`, palavra-chave ausente, perda total de pacotes, senha recusada, consulta falhada | `probe_success = 0`: a indisponibilidade é registrada, datada e contada. |
| Tudo bem | `probe_success = 1`. |

Todo monitor tem seu próprio tempo limite (`timeout_seconds`, 5 s por
padrão, 60 s no máximo), mais curto que o
`DUMBMONIT_PROBE_TIMEOUT_SECS` do servidor: interrompido pelo
agendador, ele não poderia escrever seu zero.

Regras integradas que se aplicam a todos os onze: Service down (3
minutos), Service flapping (mais de seis mudanças de estado em trinta
minutos), Slow service (mais de 3 s por dez minutos). Para toda
verificação que lê um certificado — `http`, `tls`, `smtp`, `mqtt`,
`websocket`: Certificate expiring soon (14 dias) e Certificate expired.

Uma senha recusada nunca é uma indisponibilidade do serviço: é
registrada como `reason="auth"`, então o alerta diz o que corrigir.

### Métricas {#metrics}

Todos gauges, todos com o prefixo `dumbmonit_`, todos rotulados
`probe="http|tcp|dns|ping|tls|smtp|postgres|mysql|mqtt|websocket"`
além de `target`, `host` e `tag_*`.

| Métrica | Monitores | Significado |
|---|---|---|
| `probe_success` | todos | 1 se o serviço responde corretamente, 0 caso contrário. |
| `probe_duration_seconds` | todos | Duração total da verificação. |
| `probe_failure_info` | todos | Presença (1) com um rótulo `reason`: `dns`, `connect`, `timeout`, `tls`, `cert_expired`, `status`, `keyword`, `json`, `body`, `packet_loss`, `record`, `auth`, `protocol`, `query`, `payload`. |
| `probe_http_status_code` | http, websocket | Código de status obtido. |
| `probe_http_first_byte_seconds` | http | Tempo até os cabeçalhos de resposta. |
| `probe_http_content_bytes` | http | Tamanho do corpo. |
| `probe_http_security_header{header}` | http | 0/1 por rótulo `header` (somente https): HSTS, nosniff, CSP, X-Frame-Options, frame-ancestors. |
| `probe_connect_seconds` | todos exceto dns e ping | Estabelecimento de conexão (login incluído para bancos de dados). |
| `probe_tls_handshake_seconds` | tls, http, smtp, mqtt, websocket | Negociação TLS apenas. |
| `probe_tls_version_info` | tls, http, smtp, mqtt, websocket | Presença; rótulo `version`. |
| `probe_ssl_cert_expiry_days` | tls, http, smtp, mqtt, websocket | Dias antes da expiração, negativo se expirado. |
| `probe_ssl_cert_valid` | tls, http, smtp, mqtt, websocket | 1 se a cadeia leva a uma autoridade conhecida. |
| `probe_ssl_cert_issuer_info` | tls, http, smtp, mqtt, websocket | Presença; rótulo `issuer`. |
| `probe_dns_lookup_seconds` | dns | Tempo de resolução. |
| `probe_dns_answer_records` | dns | Registros do tipo solicitado. |
| `probe_dns_answer_fingerprint` | dns | Com "Alertar quando a resposta mudar": uma impressão digital de 32 bits da resposta ordenada. Seu valor não significa nada; só suas mudanças significam. |
| `probe_icmp_rtt_seconds`, `_min_seconds`, `_max_seconds` | ping | Ida e volta médio, mais curto e mais longo. |
| `probe_icmp_packet_loss_ratio` | ping | Perda entre 0 e 1. |
| `probe_icmp_packets_sent`, `probe_icmp_packets_received` | ping | Ecos enviados e recebidos. |
| `probe_smtp_greeting_seconds` | smtp | Tempo até o banner `220`. |
| `probe_smtp_ehlo_seconds` | smtp | Ida e volta do comando `EHLO`. |
| `probe_smtp_capabilities` | smtp | Extensões anunciadas. |
| `probe_smtp_authenticated` | smtp | 1 quando `AUTH` teve sucesso. |
| `probe_sql_query_seconds` | postgres, mysql | Execução de consulta apenas. |
| `probe_sql_rows` | postgres, mysql | Linhas retornadas. |
| `probe_sql_value` | postgres, mysql | Primeira coluna da primeira linha, quando é um número. |
| `probe_mqtt_connack_seconds` | mqtt | Tempo até o `CONNACK`. |
| `probe_mqtt_suback_seconds` | mqtt | Tempo até o `SUBACK`. |
| `probe_mqtt_message_bytes` | mqtt | Tamanho da mensagem retida. |
| `probe_mqtt_message_value` | mqtt | Seu conteúdo, quando é um número. |
| `probe_ws_handshake_seconds` | websocket | Handshake de abertura. |
| `probe_ws_message_bytes` | websocket | Tamanho do frame recebido. |

## HTTP {#http}

**Site ou API web (HTTP)** — verifica que uma página ou API responde,
com o código de status certo, o conteúdo certo, e um certificado
válido. Exemplos: um site, uma página de saúde de aplicação, uma API
REST, uma interface de serviço autoalojado.

### Configuração {#setup}

1. No endereço, cole a URL completa da página a monitorar. Sem
   "http://" ou "https://", HTTPS é assumido.
2. Prefira uma página leve que não precise de login, por exemplo a
   página de saúde da aplicação ("/health", "/status"), em vez da
   página inicial.
3. Se a página exige autenticação, preencha a credencial: um nome de
   usuário e senha dão autenticação básica, um token é enviado como
   "Bearer".
4. Por padrão, qualquer código de status entre 200 e 299 está bem. Para
   ir além, exija uma palavra-chave na página ou um valor específico em
   uma resposta JSON.
5. Via HTTPS, o certificado é lido automaticamente: você será avisado
   antes de expirar.

!!! warning
    Nunca escreva uma senha ou token nas opções: eles são copiados em
    texto claro em toda medição. Use o campo de credencial, que é
    criptografado.

Credenciais: nenhuma, nome de usuário / senha (autenticação básica) ou
token de API (`Authorization: Bearer`).

### Opções {#options}

| Chave | Rótulo | Padrão | Ajuda |
|---|---|---|---|
| `method` | Método HTTP | `GET` | GET se encaixa em quase todo caso. HEAD evita baixar a página quando só o código de status importa. Escolhas: GET, HEAD, POST, PUT, PATCH, DELETE, OPTIONS. |
| `accepted_status` | Códigos de status aceitos | `200-299` | Códigos ou faixas consideradas normais, separadas por vírgulas. Qualquer outro código conta como fora do ar. Exemplo: `200-299,301,404`. |
| `keyword` | Palavra-chave esperada | *(vazio)* | Texto que precisa aparecer na página. Se estiver faltando, o serviço é reportado fora do ar mesmo que a página responda. |
| `keyword_absent` | Palavra-chave deve estar ausente | `false` | Inverte a verificação: a presença da palavra-chave sinaliza uma falha. Útil para uma página de erro que responde 200. |
| `keyword_case_sensitive` | Diferenciar maiúsculas/minúsculas | `false` | Por padrão, maiúsculas e minúsculas são tratadas igual. |
| `json_path` | Caminho JSON a verificar | *(vazio)* | Para uma resposta JSON: caminho do valor a verificar. Preencha junto com o valor esperado, nunca um sem o outro. Exemplo: `$.status`. |
| `json_expect` | Valor JSON esperado | *(vazio)* | Valor que o caminho JSON acima precisa ter, por exemplo "ok" ou "true". |
| `headers` | Cabeçalhos extras | *(vazio)* | Um ou mais cabeçalhos "Nome: valor", separados por "\|". Não coloque segredos aqui: opções são visíveis nos gráficos. |
| `body` | Corpo da requisição | *(vazio)* | Conteúdo enviado com a requisição, para métodos que esperam um (POST, PUT…). |
| `follow_redirects` | Seguir redirecionamentos | `true` | Desative para monitorar o próprio redirecionamento, por exemplo um 301 para HTTPS. |
| `max_redirects` | Máximo de redirecionamentos seguidos | `10` | Entre 1 e 20. Sem efeito se redirecionamentos não são seguidos. |
| `insecure_tls` | Aceitar um certificado não verificável | `false` | A verificação não falha mais em um certificado autoassinado ou emitido por uma autoridade privada. |
| `allow_private_targets` | Permitir alvos de loopback e link-local | `false` | Por padrão a verificação recusa endereços que só o próprio host DumbMonit pode alcançar (127.0.0.1, ::1, 169.254.x.x). Ative isso para monitorar um serviço rodando no próprio host DumbMonit. Endereços de LAN privados (10.x, 192.168.x) são sempre permitidos. |
| `check_certificate` | Ler o certificado | `true` | Via HTTPS, também registra a data de expiração do certificado para você ser avisado antes de expirar. |
| `max_body_bytes` | Máximo de bytes lidos | `524288` | Além disso, o resto da página não é baixado. A palavra-chave e o caminho JSON só são buscados nesta parte. |
| `user_agent` | Identidade anunciada (User-Agent) | `DumbMonit/<version>` | Nome que a verificação dá ao servidor. Mude se o servidor filtra robôs. |
| `timeout_seconds` | Tempo limite (segundos) | `5` | Tempo após o qual o serviço é reportado fora do ar se não respondeu. Entre 1 e 60. |

## TCP {#tcp}

**Porta de rede (TCP)** — verifica que uma porta aceita conexões: SSH,
banco de dados, compartilhamento de arquivo, servidor de jogo,
impressora de rede.

### Configuração {#setup}

1. No endereço, escreva o host seguido da porta, separados por dois
   pontos: "nas.home.lan:22" ou "192.168.1.5:445".
2. Para um endereço IPv6, coloque-o em colchetes: "[fd00::1]:445".
3. Você também pode deixar o endereço sem porta e informá-la na opção
   "Porta": um dos dois é obrigatório.
4. Só a abertura da conexão é testada: nenhum dado é enviado ao
   serviço, então não tem efeito sobre ele.

!!! warning
    Uma porta aberta não prova que a aplicação detrás dela funciona.
    Para um serviço web, prefira o tipo "Site ou API web", que lê a
    resposta.

### Opções {#options}

| Chave | Rótulo | Padrão | Ajuda |
|---|---|---|---|
| `port` | Porta | *(vazio)* | Porta a abrir, se o endereço não já der como "host:porta". Um dos dois é obrigatório. |
| `allow_private_targets` | Permitir alvos de loopback e link-local | `false` | Por padrão a verificação recusa endereços que só o próprio host DumbMonit pode alcançar (127.0.0.1, ::1, 169.254.x.x). Ative isso para monitorar um serviço rodando no próprio host DumbMonit. Endereços de LAN privados (10.x, 192.168.x) são sempre permitidos. |
| `timeout_seconds` | Tempo limite (segundos) | `5` | Tempo após o qual o serviço é reportado fora do ar se não respondeu. Entre 1 e 60. |

## DNS {#dns}

**Nome de domínio (DNS)** — verifica que um nome resolve, e que aponta
para o endereço certo. Exemplos: seu nome de domínio, um nome interno
servido pelo seu Pi-hole ou AdGuard, um registro MX.

### Configuração {#setup}

1. No endereço, escreva o nome a resolver, sem "http://":
   "www.example.com".
2. Por padrão, a verificação pede ao resolvedor do sistema um endereço
   IPv4 (registro A). Escolha outro tipo de registro nas opções se
   necessário.
3. Para monitorar seu próprio servidor DNS, informe seu endereço IP em
   "Servidor DNS a consultar": a verificação falhará se ele parar de
   responder.
4. Para detectar sequestro ou uma má configuração, liste em "Valores
   esperados" os endereços que a resposta precisa conter.

!!! warning
    O servidor DNS é dado por seu endereço IP, não por um nome: seria
    necessário um resolvedor para resolver o resolvedor.

### Opções {#options}

| Chave | Rótulo | Padrão | Ajuda |
|---|---|---|---|
| `record_type` | Tipo de registro | `A` | A para um endereço IPv4, AAAA para IPv6, MX para e-mail, CNAME para um alias… Escolhas: A, AAAA, CNAME, MX, TXT, NS, SOA, SRV, PTR, CAA. |
| `resolver` | Servidor DNS a consultar | *(vazio)* | Endereço IP de um resolvedor, porta opcional. Vazio: o resolvedor do sistema. Útil para verificar seu próprio servidor DNS. Exemplo: `1.1.1.1` ou `10.0.0.1:5353`. |
| `expect` | Valores esperados | *(vazio)* | Valores que todos precisam aparecer na resposta, separados por vírgulas. Vazio: só a resolução é verificada. |
| `expect_mode` | Comparação | `contains` | `contains`: cada valor esperado precisa aparecer em algum lugar na resposta. `exact`: a resposta precisa conter esses valores e nada mais. |
| `forbid` | Valores proibidos | *(vazio)* | Valores que nunca devem aparecer na resposta, separados por vírgulas. Verificado antes dos esperados. |
| `alert_on_change` | Alertar quando a resposta mudar | `false` | Registra uma impressão digital da resposta ordenada e alerta sempre que muda, sem precisar escrever os valores esperados. |
| `timeout_seconds` | Tempo limite (segundos) | `5` | Tempo após o qual o serviço é reportado fora do ar se não respondeu. Entre 1 e 60. |

### Afirmando o que uma zona responde {#asserting-what-a-zone-answers}

`expect` por si só pega o registro que desapareceu. Não pega o
registro que foi *adicionado*: uma zona sequestrada frequentemente
mantém o `A` legítimo e coloca um segundo ao lado, e um cliente então
alcança um dos dois aleatoriamente.

- `expect_mode = exact` recusa qualquer valor que a expectativa não
  cobre. A ordem não importa — o DNS não mantém uma.
- `forbid` nomeia valores que nunca devem voltar: o endereço de um host
  antigo, um `TXT` de validação expirado, um servidor de e-mail que
  você desativou. É verificado primeiro, então vence sobre `expect`.

Ambos comparam sem diferenciar maiúsculas/minúsculas, ignoram o ponto
final de um nome e as aspas em torno de um `TXT`, e tratam um valor
esperado como encontrado quando aparece dentro de um registro — então
um `MX` corresponde em `mail.example.com` sem você ter que copiar sua
prioridade.

### Alertando quando a resposta muda {#alerting-when-the-answer-changes}

Escrever os valores esperados não é sempre prático: um nome servido por
um CDN, registros que outra pessoa gerencia, ou simplesmente uma zona
sobre a qual você quer ser avisado sempre que se move. Marque
**Alertar quando a resposta mudar**: toda verificação então registra
uma impressão digital da resposta, e a regra integrada
[DNS answer changed](../alerting/rules.md#reverse-proxies-and-domains)
dispara quando difere da anterior. O alerta se resolve por conta
própria uma hora depois; a nova resposta é a referência a partir de
então.

A impressão digital cobre os valores da resposta, ordenados e sem
duplicatas, com a cadeia CNAME incluída: um round-robin que retorna os
mesmos endereços em outra ordem não a muda, nem um TTL. Um nome cuja
resposta muda em toda consulta (um pool grande como `pool.ntp.org`, ou
DNS geográfico que responde diferente de um resolvedor para outro)
alertaria o tempo todo: deixe a opção desligada para esses, ou consulte
um resolvedor fixo.

## Ping {#ping}

**Host alcançável (ping)** — envia alguns ecos ICMP e mede o tempo de
resposta e perda de pacotes. Exemplos: roteador ou gateway doméstico,
ponto de acesso Wi-Fi, impressora, máquina sem agente ou SNMP, host
remoto.

### Configuração {#setup}

1. No endereço, escreva o nome de host ou endereço IP: "192.168.1.1" ou
   "router.home.lan".
2. A cada sondagem, a verificação envia quatro ecos e mede o tempo de
   resposta, junto com a parcela de pacotes perdidos.
3. Por padrão, só uma perda total conta como fora do ar; perda parcial
   permanece visível nos gráficos. Diminua "Perda tolerada" para ser
   avisado antes.
4. Se o DumbMonit roda no Docker, mantenha as linhas `sysctls:` do
   docker-compose.yml entregue sob o serviço dumbmonit: elas permitem
   que o usuário sem privilégios do contêiner abra sockets de eco
   ICMP. Nenhuma capacidade é necessária.

!!! warning
    Sem esse sysctl, a verificação não pode abrir um socket ICMP: ela
    reporta isso como um erro de configuração, não como uma falha de
    host. Alguns monitores também ignoram pings de propósito:
    verifique isso antes de tirar conclusões.

```yaml
services:
  dumbmonit:
    sysctls:
      net.ipv4.ping_group_range: "0 2147483647"
```

Fora do Docker, `sysctl -w net.ipv4.ping_group_range="0 2147483647"` no
host faz a mesma coisa, e um socket bruto (`NET_RAW`, root) também
funciona.

### Opções {#options}

| Chave | Rótulo | Padrão | Ajuda |
|---|---|---|---|
| `count` | Número de ecos | `4` | Pacotes enviados a cada sondagem, de 1 a 20. |
| `packet_timeout_ms` | Espera por pacote (ms) | `1000` | Tempo permitido para cada resposta, de 50 a 10.000 ms. |
| `interval_ms` | Intervalo entre dois ecos (ms) | `100` | Pausa entre dois ecos, de 0 a 5.000 ms. |
| `payload_bytes` | Tamanho do payload (bytes) | `56` | Dados carregados por cada eco, de 0 a 1.400 bytes. |
| `ip_version` | Versão IP | `auto` | "auto" toma o primeiro endereço resolvido; force 4 ou 6 se o host tem ambos e um não responde. Escolhas: auto, 4, 6. |
| `loss_threshold_percent` | Perda tolerada (%) | `100` | Acima desta porcentagem de pacotes perdidos, o host é reportado fora do ar. 100: só uma perda total conta, perda parcial permanece visível nos gráficos. |
| `timeout_seconds` | Tempo limite (segundos) | `5` | Orçamento total para a sondagem, de 1 a 60. Recusado se não cobrir "número de ecos × espera por pacote". |

## TLS {#tls}

**Certificado TLS** — verifica que um certificado é válido e avisa
antes de expirar, em qualquer porta criptografada. Exemplos: servidor
de e-mail (IMAPS, SMTPS), proxy reverso, diretório LDAPS, broker MQTT,
interface de administração.

### Configuração {#setup}

1. No endereço, escreva o host e, se não for 443, a porta do serviço
   criptografado: "mail.example.com:993", "ldap.home.lan:636".
2. A verificação abre uma conexão criptografada, lê o certificado
   apresentado e fecha imediatamente: nenhum dado é trocado com a
   aplicação.
3. Registra o número de dias antes da expiração, o emissor e a versão
   TLS. As regras de alerta empacotadas avisam em quatorze dias, depois
   na expiração.
4. Se o serviço é alcançado por seu endereço IP mas o certificado
   carrega um nome, informe esse nome em "Nome do servidor (SNI)".
5. Para um certificado emitido por sua própria autoridade, marque
   "Aceitar um certificado não verificável": a data de expiração ainda
   é monitorada.

!!! warning
    Para um site, o tipo "Site ou API web" já lê o certificado: este
    tipo é pensado para serviços que não falam HTTP, ou cuja aplicação
    você não quer atingir.

O handshake se completa mesmo quando o certificado é recusado: é isso
que permite ler a data de expiração de um certificado que já expirou. O
veredito da verificação está em `probe_ssl_cert_valid`.

### Opções {#options}

| Chave | Rótulo | Padrão | Ajuda |
|---|---|---|---|
| `server_name` | Nome do servidor (SNI) | *(vazio)* | Nome de domínio anunciado ao servidor e verificado no certificado. Defina quando o endereço é um IP detrás de um proxy reverso. |
| `insecure_tls` | Aceitar um certificado não verificável | `false` | Uma cadeia não verificável não conta mais como falha: a data de expiração ainda é registrada. |
| `allow_private_targets` | Permitir alvos de loopback e link-local | `false` | Por padrão a verificação recusa endereços que só o próprio host DumbMonit pode alcançar (127.0.0.1, ::1, 169.254.x.x). Ative isso para monitorar um serviço rodando no próprio host DumbMonit. Endereços de LAN privados (10.x, 192.168.x) são sempre permitidos. |
| `timeout_seconds` | Tempo limite (segundos) | `5` | Tempo após o qual o serviço é reportado fora do ar se não respondeu. Entre 1 e 60. |

## SMTP {#smtp}

**Relay de e-mail (SMTP)** — abre uma sessão real em um servidor de
e-mail: saudação, EHLO, STARTTLS, e o login se você der um. Exemplos:
o relay SMTP do seu provedor, um Postfix ou msmtp local, Proxmox Mail
Gateway, a porta de submissão de um servidor de e-mail.

### Configuração {#setup}

1. No endereço, escreva o servidor de e-mail, e a porta se não for a
   usual: "smtp.example.com" ou "smtp.example.com:2525".
2. Escolha a criptografia que seu relay usa: STARTTLS para a porta de
   submissão 587, TLS para a porta 465, Nenhuma para um relay na sua
   própria rede escutando na porta 25.
3. Para verificar que o relay ainda aceita sua conta, preencha a
   credencial: a verificação então executa AUTH e reporta uma senha
   recusada como tal, não como uma indisponibilidade.
4. Nada é jamais enviado: a verificação para depois da saudação, das
   extensões e do login opcional, depois desliga. Nenhuma mensagem
   entra na fila.
5. Para ser avisado quando uma extensão desaparece, nomeie-a em
   "Extensão esperada": um relay que para de anunciar STARTTLS é um
   relay que enviaria seu e-mail em texto claro.

!!! warning
    Uma sessão aceita não prova que o e-mail sai. Um relay que aceita
    tudo e enfileira para sempre responde perfeitamente aqui: observe
    as próprias filas para isso.

Credenciais: nenhuma, ou um nome de usuário e senha enviados com
`AUTH PLAIN` (ou `AUTH LOGIN` quando o servidor oferece só isso). Eles
são enviados **depois** da conexão ser criptografada e depois do
certificado ser julgado: um relay cuja cadeia não pode ser verificada
nunca os recebe.

Via TLS ou STARTTLS, o certificado é lido como a verificação TLS o
lê — expiração, emissor, versão de protocolo — então um relay de e-mail
também é um certificado sobre o qual você é avisado.

### O que ele não consegue detectar {#what-it-cannot-detect}

Que o e-mail realmente saia. Um relay que aceita toda sessão e depois
adia tudo responde perfeitamente aqui; observe suas filas em vez disso
(veja a página do Proxmox Mail Gateway). Também nunca envia uma
mensagem, então não diz nada sobre filtragem de destinatário, SPF ou
DKIM.

### Opções {#options}

| Chave | Rótulo | Padrão | Ajuda |
|---|---|---|---|
| `security` | Criptografia | `starttls` | STARTTLS começa em texto claro e eleva (porta 587), TLS criptografa desde o primeiro byte (porta 465), Nenhuma nunca criptografa (porta 25) e só serve para um relay na sua própria rede. |
| `port` | Porta | *(vazio)* | Usada se o endereço não der uma. Vazio: 587 com STARTTLS, 465 com TLS, 25 sem criptografia. |
| `helo_name` | Nome anunciado (EHLO) | `dumbmonit` | Nome que a verificação dá ao se apresentar. Um relay estrito recusa um nome que não consegue resolver. |
| `expect_capability` | Extensão esperada | *(vazio)* | Extensão que o servidor precisa anunciar em sua resposta EHLO, por exemplo STARTTLS ou AUTH. Vazio: nenhuma expectativa. |
| `server_name` | Nome do servidor (SNI) | *(o host)* | Nome de domínio anunciado ao servidor e verificado no certificado. Defina quando o endereço é um IP. |
| `insecure_tls` | Aceitar um certificado não verificável | `false` | Uma cadeia não verificável não conta mais como falha: a data de expiração ainda é registrada. |
| `allow_private_targets` | Permitir alvos de loopback e link-local | `false` | Veja "Endereços que as verificações recusam" abaixo. |
| `timeout_seconds` | Tempo limite (segundos) | `5` | Tempo após o qual o serviço é reportado fora do ar se não respondeu. Entre 1 e 60. |

### Motivos de falha {#failure-reasons}

| `reason` | O que aconteceu |
|---|---|
| `connect` | A porta está fechada ou o relay é inacessível. |
| `protocol` | Algo respondeu, mas não SMTP — ou recusou a conexão, `EHLO`, ou o STARTTLS que anunciou. |
| `auth` | A conta foi recusada (`535`, `530`, `534`). O relay está bem; a credencial não. |
| `payload` | A extensão nomeada em "Extensão esperada" não é anunciada. |
| `tls`, `cert_expired` | Mesmo significado que a verificação TLS. |

## PostgreSQL {#postgresql}

**Banco de dados PostgreSQL** — conecta, autentica e executa uma
consulta: o sinal para um banco de dados que está ativo mas não
responde mais. Exemplos: o banco de dados detrás do Nextcloud ou
Immich, um recorder do Home Assistant, um banco de dados de aplicação,
uma instância TimescaleDB.

### Configuração {#setup}

1. No servidor, crie uma conta para monitoramento e não dê a ela nada
   além do direito de conectar: CREATE ROLE dumbmonit LOGIN PASSWORD
   'a-long-password';
2. Certifique-se de que essa conta pode alcançar o servidor a partir da
   máquina DumbMonit, o que geralmente significa mais uma linha em
   pg_hba.conf seguida de um recarregamento de configuração.
3. No endereço, escreva o servidor, e a porta se não for 5432:
   "db.home.lan" ou "db.home.lan:5433".
4. A consulta padrão é SELECT 1, que não lê nenhuma tabela: nada mais
   precisa ser concedido. Tempo de conexão e tempo de consulta são
   registrados separadamente.
5. Para observar algo próprio, substitua a consulta por uma que retorna
   um único número, e ela se torna um gráfico: um tamanho de fila, uma
   contagem de linhas, um atraso de replicação.

!!! warning
    A verificação abre uma conexão real a cada sondagem. Em uma
    instância já próxima de seu limite de conexões, dê a ela um
    intervalo mais longo.

Credenciais: um nome de usuário e senha, obrigatório. Um banco de dados
nunca é monitorado anonimamente, então o formulário não oferece nada
mais.

### O que ele não consegue detectar {#what-it-cannot-detect}

Atraso de replicação, um bloqueio que se arrasta, uma tabela que
continua crescendo. Tudo isso vive nas próprias visões de administração
do motor — escreva a consulta você mesmo, e o número que ela retorna se
torna `probe_sql_value`, um gráfico sobre o qual você pode alertar.

### Opções {#options}

| Chave | Rótulo | Padrão | Ajuda |
|---|---|---|---|
| `port` | Porta | `5432` | Usada se o endereço não der uma. |
| `database` | Banco de dados | `postgres` | Banco de dados aberto na conexão. A conta de monitoramento precisa ter permissão de conectar a ele. |
| `query` | Consulta | `SELECT 1` | Executada a cada verificação. A padrão não lê nenhuma tabela, então a conta não precisa de nenhum privilégio além de conectar. Uma consulta retornando um número a transforma em um gráfico. |
| `expect` | Valor esperado | *(vazio)* | Valor que a primeira coluna da primeira linha precisa ter. Vazio: só o sucesso da consulta é verificado. |
| `sslmode` | Criptografia | `prefer` | `prefer` criptografa quando o servidor oferece, `require` recusa conectar sem isso, `verify-full` também verifica o certificado e o nome, `disable` nunca criptografa. |
| `allow_private_targets` | Permitir alvos de loopback e link-local | `false` | Veja "Endereços que as verificações recusam" abaixo. |
| `timeout_seconds` | Tempo limite (segundos) | `5` | Tempo após o qual o serviço é reportado fora do ar se não respondeu. Entre 1 e 60. |

## MySQL e MariaDB {#mysql-and-mariadb}

**Banco de dados MySQL ou MariaDB** — conecta, autentica e executa uma
consulta: o sinal para um banco de dados que está ativo mas não
responde mais. Exemplos: o banco de dados detrás de um WordPress, um
Nextcloud ou um Kimai, um banco de dados de aplicação, um MariaDB no
Docker.

### Configuração {#setup}

1. No servidor, crie uma conta para monitoramento sem nenhum
   privilégio: CREATE USER 'dumbmonit'@'%' IDENTIFIED BY
   'a-long-password';
2. Verifique que a conta pode conectar a partir da máquina DumbMonit:
   um padrão de host localhost só aceitaria conexões feitas no próprio
   servidor.
3. No endereço, escreva o servidor, e a porta se não for 3306:
   "db.home.lan" ou "db.home.lan:3307".
4. A consulta padrão é SELECT 1, que não lê nenhuma tabela: nada mais
   precisa ser concedido. Tempo de conexão e tempo de consulta são
   registrados separadamente.
5. Para observar algo próprio, substitua a consulta por uma que retorna
   um único número, e ela se torna um gráfico: um tamanho de fila, uma
   contagem de linhas, um atraso de replicação.

!!! warning
    A verificação abre uma conexão real a cada sondagem. Em uma
    instância já próxima de seu limite de conexões, dê a ela um
    intervalo mais longo.

### Opções {#options}

| Chave | Rótulo | Padrão | Ajuda |
|---|---|---|---|
| `port` | Porta | `3306` | Usada se o endereço não der uma. |
| `database` | Banco de dados | *(vazio)* | Banco de dados aberto na conexão. Vazio: nenhum, que é suficiente para a consulta padrão. |
| `query` | Consulta | `SELECT 1` | Executada a cada verificação. A padrão não lê nenhuma tabela, então a conta não precisa de nenhum privilégio além de conectar. Uma consulta retornando um número a transforma em um gráfico. |
| `expect` | Valor esperado | *(vazio)* | Valor que a primeira coluna da primeira linha precisa ter. Vazio: só o sucesso da consulta é verificado. |
| `sslmode` | Criptografia | `prefer` | `prefer` criptografa quando o servidor oferece, `require` recusa conectar sem isso, `verify-full` também verifica o certificado e o nome, `disable` nunca criptografa. |
| `allow_private_targets` | Permitir alvos de loopback e link-local | `false` | Veja "Endereços que as verificações recusam" abaixo. |
| `timeout_seconds` | Tempo limite (segundos) | `5` | Tempo após o qual o serviço é reportado fora do ar se não respondeu. Entre 1 e 60. |

### Motivos de falha, ambos os motores {#failure-reasons-both-engines}

| `reason` | O que aconteceu |
|---|---|
| `connect` | A porta está fechada, o servidor é inacessível, ou recusou a conexão por um motivo que não é a conta (iniciando, muitas conexões). |
| `auth` | A conta foi recusada: senha errada, nenhuma linha `pg_hba.conf`, padrão de host que não cobre a máquina DumbMonit. |
| `query` | Conectou e autenticou, mas a consulta falhou: tabela faltando, banco de dados somente leitura, erro de sintaxe. |
| `payload` | A consulta funcionou mas retornou algo diferente do valor esperado. |
| `timeout` | A conexão ou a consulta não terminou a tempo — o caso "ativo mas lento". |

## MQTT {#mqtt}

**Broker MQTT** — conecta ao broker, se inscreve em um tópico, e pode
esperar por uma mensagem retida. Exemplos: Mosquitto, o broker detrás
do Home Assistant, Zigbee2MQTT, sensores ESPHome.

### Configuração {#setup}

1. No endereço, escreva o broker, e a porta se não for a usual:
   "broker.home.lan" ou "broker.home.lan:1884".
2. Marque "Conexão criptografada" para um broker escutando na 8883, e
   preencha a credencial se exigir uma conta.
3. Conectar já vale a pena monitorar: um broker que recusa sua conta,
   ou que parou de responder, é o motivo de a casa inteira ficar
   quieta.
4. Para ir além, nomeie um tópico: a verificação então se inscreve
   nele, e um broker que recusa a inscrição diz que a conta perdeu seus
   direitos de acesso.
5. Marque "Esperar uma mensagem retida" só para um tópico que carrega
   uma, tipicamente um tópico de disponibilidade. Um tópico publicado
   de tempos em tempos parece silencioso para um cliente que acabou de
   conectar.

!!! warning
    A verificação nunca publica nada. Também não pode dizer quão
    antiga é uma mensagem retida: o MQTT não as data, então um valor
    congelado há três semanas ainda conta como presente.

### O que ele não consegue detectar {#what-it-cannot-detect}

Que um determinado sensor parou de emitir. A mensagem retida que a
verificação lê é o que quer que o broker manteve, possivelmente com
três semanas — o MQTT não a marca com data. Para atualidade, observe a
automação que a consome, ou use um heartbeat.

### Opções {#options}

| Chave | Rótulo | Padrão | Ajuda |
|---|---|---|---|
| `tls` | Conexão criptografada | `false` | Criptografa a sessão desde o primeiro byte, como brokers fazem na porta 8883. |
| `port` | Porta | *(vazio)* | Usada se o endereço não der uma. Vazio: 1883 em texto claro, 8883 criptografada. |
| `topic` | Tópico | *(vazio)* | Tópico ao qual a verificação se inscreve. Vazio: só conecta, o que já diz que o broker está ativo e aceita sua conta. |
| `expect_message` | Esperar uma mensagem retida | `false` | Uma mensagem precisa chegar nesse tópico antes do tempo limite. Só mensagens retidas chegam imediatamente: um tópico que é apenas publicado de tempos em tempos parecerá silencioso. |
| `expect` | Conteúdo esperado | *(vazio)* | Texto que a mensagem precisa conter. Preenchê-lo implica esperar uma mensagem. |
| `client_id` | Identificador de cliente | *(derivado)* | Nome anunciado ao broker. O padrão deriva do monitor, então duas verificações nunca se desconectam. |
| `server_name` | Nome do servidor (SNI) | *(o host)* | Nome de domínio anunciado ao broker e verificado no certificado. Defina quando o endereço é um IP. |
| `insecure_tls` | Aceitar um certificado não verificável | `false` | Uma cadeia não verificável não conta mais como falha: a data de expiração ainda é registrada. |
| `allow_private_targets` | Permitir alvos de loopback e link-local | `false` | Veja "Endereços que as verificações recusam" abaixo. |
| `timeout_seconds` | Tempo limite (segundos) | `5` | Tempo após o qual o serviço é reportado fora do ar se não respondeu. Entre 1 e 60. |

### Motivos de falha {#failure-reasons}

| `reason` | O que aconteceu |
|---|---|
| `connect` | A porta está fechada ou o broker é inacessível. |
| `auth` | O `CONNACK` recusou a conta, ou o broker recusou a inscrição — uma lista de controle de acesso não cobre mais esse tópico. |
| `protocol` | Algo respondeu mas não fala MQTT 3.1.1, ou o broker rejeitou o identificador de cliente. |
| `payload` | Nenhuma mensagem retida chegou no tópico, ou seu conteúdo não contém o texto esperado. |

## WebSocket {#websocket}

**Endpoint WebSocket** — executa o handshake de upgrade, e pode enviar
um frame e esperar por um: uma falha diferente de uma página HTTP
simples. Exemplos: a API do Home Assistant, um painel ao vivo, um
fluxo de log, um endpoint detrás de um proxy reverso.

### Configuração {#setup}

1. No endereço, cole o endpoint completo:
   "wss://home.example.com/api/websocket". Sem um esquema, wss é
   assumido.
2. A verificação executa todo o handshake de abertura e verifica a
   resposta que o servidor calcula a partir da chave que foi enviada:
   um proxy reverso que perdeu o cabeçalho Upgrade é pego aqui, onde
   uma verificação HTTP ainda veria um 200 saudável.
3. Nada mais é necessário para a maioria dos endpoints. Um status
   diferente de 101 é reportado com seu código, então um 401 de um
   token expirado não é confundido com uma indisponibilidade.
4. Para ir além, preencha "Conteúdo esperado" com um trecho do primeiro
   frame que o servidor envia por conta própria, por exemplo a
   saudação da API do Home Assistant.
5. Se o endpoint não diz nada até ser falado com, coloque uma mensagem
   em "Frame a enviar" e o que sua resposta precisa conter em
   "Conteúdo esperado".

!!! warning
    A verificação abre e desliga dentro de um segundo. Uma conexão que
    um firewall ou um tempo limite de proxy corta depois de trinta
    segundos parece perfeitamente saudável aqui.

Credenciais: nenhuma, um nome de usuário e senha (enviados como
autenticação básica HTTP na requisição de abertura) ou um token
(enviado como `Authorization: Bearer`).

### Opções {#options}

| Chave | Rótulo | Padrão | Ajuda |
|---|---|---|---|
| `path` | Caminho | *(do endereço)* | Caminho da requisição de abertura. Tomado do endereço quando ele carrega um. |
| `port` | Porta | *(vazio)* | Usada se o endereço não der uma. Vazio: 443 para wss, 80 para ws. |
| `send` | Frame a enviar | *(vazio)* | Frame de texto enviado uma vez que a conexão está aberta. Vazio: nada é enviado. |
| `expect` | Conteúdo esperado | *(vazio)* | Texto que um frame recebido precisa conter. Vazio e sem nada a enviar, só o handshake de abertura é verificado. |
| `subprotocol` | Subprotocolo | *(vazio)* | Valor de Sec-WebSocket-Protocol. A verificação falha se o servidor escolhe um diferente. |
| `origin` | Origem | *(vazio)* | Valor do cabeçalho Origin, que alguns servidores exigem antes de elevar. |
| `server_name` | Nome do servidor (SNI) | *(o host)* | Nome de domínio anunciado ao servidor e verificado no certificado. Defina quando o endereço é um IP. |
| `insecure_tls` | Aceitar um certificado não verificável | `false` | Uma cadeia não verificável não conta mais como falha: a data de expiração ainda é registrada. |
| `allow_private_targets` | Permitir alvos de loopback e link-local | `false` | Veja "Endereços que as verificações recusam" abaixo. |
| `timeout_seconds` | Tempo limite (segundos) | `5` | Tempo após o qual o serviço é reportado fora do ar se não respondeu. Entre 1 e 60. |

### Motivos de falha {#failure-reasons}

| `reason` | O que aconteceu |
|---|---|
| `connect` | A porta está fechada ou o host é inacessível. |
| `status` | O endpoint respondeu algo diferente de `101`; `probe_http_status_code` carrega o código. |
| `protocol` | Respondeu `101` mas seu `Sec-WebSocket-Accept` não corresponde à chave enviada, escolheu outro subprotocolo, ou fechou a conexão imediatamente. |
| `payload` | Nenhum frame chegou, ou não contém o texto esperado. |

## NTP {#ntp}

**Servidor de hora NTP** — envia uma única requisição SNTP e lê a
resposta: desvio, atraso de ida e volta, stratum e indicador de salto.
Exemplos: um roteador ou NAS agindo como servidor de hora local, um
servidor NTP interno (chrony, ntpd), um servidor de pool público usado
como referência.

### Configuração {#setup}

1. No endereço, escreva o nome ou endereço IP do servidor:
   "pool.ntp.org" ou "192.168.1.1". A porta UDP 123 é usada a menos que
   a opção "Porta" diga o contrário.
2. A verificação envia uma única requisição SNTP e lê a resposta:
   nenhum estado é mantido entre sondagens, e nada é escrito no
   servidor.
3. Registra o desvio entre o relógio do servidor e o host DumbMonit, o
   atraso de ida e volta, o stratum e o indicador de salto.
4. Um servidor que responde stratum 16, ou cujo indicador de salto lê
   "não sincronizado", nunca conseguiu sincronizar: a verificação o
   reporta fora do ar qualquer que seja o desvio.
5. Diminua "Desvio tolerado" para ser avisado antes de 100 ms; aumente
   para um servidor vários saltos de um relógio de referência.

!!! warning
    O desvio é só tão bom quanto o próprio relógio do host DumbMonit:
    se esse host não é ele mesmo mantido sincronizado (NTP, a
    sincronização de hora de um hipervisor), o desvio medido reflete
    sua deriva tanto quanto a do servidor.

Credenciais: nenhuma.

### Opções {#options}

| Chave | Rótulo | Padrão | Ajuda |
|---|---|---|---|
| `port` | Porta | `123` | Porta UDP do servidor NTP. |
| `offset_threshold_ms` | Desvio tolerado (ms) | `100` | Acima desse desvio de relógio, o servidor é reportado fora do ar. Um servidor não sincronizado (stratum 16) sempre falha, qualquer que seja esse valor. |
| `allow_private_targets` | Permitir alvos de loopback e link-local | `false` | Veja "Endereços que as verificações recusam" abaixo. |
| `timeout_seconds` | Tempo limite (segundos) | `5` | Tempo após o qual o serviço é reportado fora do ar se não respondeu. Entre 1 e 60. |

### Motivos de falha {#failure-reasons}

| `reason` | O que aconteceu |
|---|---|
| `connect` | O socket UDP não pôde ser aberto, ou a resposta veio de um endereço inesperado. |
| `timeout` | Nenhuma resposta chegou dentro do tempo limite. |
| `protocol` | A resposta não era um pacote NTP válido, ou não veio de um servidor (modo errado). |
| `unsynchronized` | O servidor respondeu stratum 16 ou 0 (um "kiss-o'-death", que também nomeia o motivo no log), ou seu indicador de salto lê "não sincronizado". |
| `clock_offset` | O desvio medido excede "Desvio tolerado". |

## Endereços que as verificações recusam {#addresses-the-checks-refuse}

As verificações que se conectam a um endereço que você dá a elas —
HTTP, TCP, TLS, NTP, SMTP, PostgreSQL, MySQL, MQTT, WebSocket — reportam
o que veem — que também é, palavra por palavra, o que uma falsificação
de requisição do lado do servidor faz. Para impedir que um
administrador de monitoramento leia serviços que só o host DumbMonit
pode alcançar (o banco de dados de métricas embutido em
`127.0.0.1:8428`, o serviço de metadados de um provedor de nuvem em
`169.254.169.254`), as verificações recusam por padrão:

- endereços loopback (`127.0.0.0/8`, `::1`);
- endereços link-local (`169.254.0.0/16`, `fe80::/10`), incluindo o
  endereço de metadados de nuvem, e o endereço não especificado
  (`0.0.0.0`, `::`).

Faixas de LAN privada (`10.0.0.0/8`, `172.16.0.0/12`, `192.168.0.0/16`,
`fc00::/7`) são sempre permitidas: monitorá-las é o ponto de um
homelab.

A regra se aplica ao endereço que você digitou, a todo endereço para o
qual um nome de host resolve (verificado de novo no momento da
conexão, então um nome que muda sua resposta não pode escapar), e a
todo redirecionamento que a verificação HTTP segue. Um endereço
recusado aparece como um erro de configuração no alvo — nenhum alerta
de fora do ar é enviado — nomeando a opção a ativar:
`allow_private_targets`. Ative-a para um alvo que realmente monitora um
serviço no próprio host DumbMonit.

Um redirecionamento para um endereço recusado é reportado como uma
medição falhada (`reason="connect"`), porque é o serviço monitorado que
mudou, não a configuração.

Valores encontrados em um `json_path` nunca são ecoados além de 32
caracteres em uma mensagem de falha.

## Erros comuns {#common-errors}

| Sintoma | Causa provável |
|---|---|
| Ping mostra um erro de configuração | O sysctl `net.ipv4.ping_group_range` está faltando no arquivo Compose. Veja acima. |
| HTTP fora do ar com `reason="keyword"` | A palavra-chave só é buscada nos primeiros `max_body_bytes`, sem diferenciar maiúsculas/minúsculas a menos que `keyword_case_sensitive` esteja definido. |
| HTTP fora do ar com `reason="tls"` | Autoassinado ou autoridade privada: defina `insecure_tls`. |
| Uma verificação mostra um erro de configuração nomeando `allow_private_targets` | O endereço é loopback ou link-local. Veja "Endereços que as verificações recusam" acima. |
| TCP fora do ar mesmo que `telnet host porta` funcione do seu laptop | O contêiner DumbMonit não consegue alcançar o host (rede Docker, firewall). Teste de dentro da rede do contêiner. |
| DNS fora do ar com `reason="record"` | A resposta não contém todo valor listado em `expect`. |
| Ping fora do ar com `reason="packet_loss"` | Perda acima de `loss_threshold_percent`; alguns monitores limitam a taxa de ICMP. |
| DNS fora do ar com `reason="record"` mesmo que `expect` corresponda | `expect_mode` é `exact` e a resposta contém um valor extra, ou `forbid` correspondeu. |
| SMTP fora do ar com `reason="protocol"` em `security = starttls` | O relay não anuncia STARTTLS nessa porta. Use `tls` para 465, `none` para um relay local simples. |
| Uma verificação de banco de dados fora do ar com `reason="auth"` | A conta existe mas pode não conectar da máquina DumbMonit: verifique `pg_hba.conf`, ou a parte de host do usuário MySQL. |
| Uma verificação de banco de dados fora do ar com `reason="query"` | Conexão e login funcionaram. O banco de dados está ativo e recusando trabalhar: somente leitura, tabela faltando, recuperação. |
| MQTT fora do ar com `reason="payload"` em um tópico que claramente tem tráfego | Só mensagens *retidas* chegam a um cliente que acabou de se inscrever. Publique esse tópico com a flag retain, ou desmarque "Esperar uma mensagem retida". |
| WebSocket fora do ar com `reason="protocol"` depois de um `101` | Algo no caminho responde o upgrade sem falar WebSocket — geralmente uma regra de proxy reverso que engole o cabeçalho `Upgrade`. |
