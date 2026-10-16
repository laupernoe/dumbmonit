# Nginx {#nginx}

O servidor web e proxy inverso: ligações ativas e em espera, as taxas de
aceitação/tratamento/pedidos, e a saúde dos upstreams quando a API comercial
do NGINX Plus está disponível.

O DumbMonit lê o módulo `stub_status` incorporado do Nginx, um conjunto de
contadores em texto simples, sem necessidade de API nem de conta numa rede
corretamente filtrada. Quando o NGINX Plus comercial está instalado, o
DumbMonit também pode ler a sua API JSON para a saúde dos upstreams e
contadores por zona.

## O que monitoriza {#what-it-watches}

Todas as métricas têm o prefixo `dumbmonit_nginx_`. As métricas `_total` são
contadores: os gráficos e as regras convertem-nas em taxas.

| Métrica | O quê | Etiquetas |
|---|---|---|
| `connections_active` | ligações atualmente abertas | |
| `connections_reading`, `connections_writing`, `connections_waiting` | ligações pelo que estão a fazer | |
| `accepts_total`, `handled_total` | ligações aceites, e efetivamente tratadas | |
| `requests_total` | pedidos servidos desde que o Nginx arrancou | |
| `version_info` | presença; o cabeçalho de resposta `Server`, quando o Nginx o envia | `server` |
| `plus_upstream_server_up` | 1 quando o NGINX Plus marca o servidor como ativo | `upstream`, `server` |
| `plus_upstream_server_active`, `plus_upstream_server_requests_total`, `plus_upstream_server_fails_total`, `plus_upstream_server_5xx_total` | contadores por servidor do NGINX Plus | `upstream`, `server` |
| `plus_zone_requests_total`, `plus_zone_5xx_total`, `plus_zone_received_bytes_total`, `plus_zone_sent_bytes_total` | contadores por zona do NGINX Plus | `zone` |

Uma diferença entre `accepts_total` e `handled_total` significa ligações que
o Nginx descartou antes de as poder servir (geralmente `worker_connections`
atingido). As métricas `plus_*` só aparecem quando «Read the NGINX Plus API»
está marcado e o NGINX Plus responde; a versão open-source não tem um
«upstream» para reportar só com o `stub_status`.

## A página do monitor {#the-device-page}

O painel acima dos gráficos lê o que a verificação guardou; abrir a página
nunca consulta o Nginx. Mostra ligações ativas, de leitura, de escrita e em
espera, as taxas de aceitação/tratamento/pedidos ao longo de 24 horas, e, com
o NGINX Plus, a saúde dos upstreams e o tráfego por zona.

## Ativar o módulo stub_status do Nginx {#enable-nginxs-stub_status-module}

1. O Nginx vem com o stub_status incorporado: só precisa de uma localização que o ative. Adicione isto ao bloco de servidor que o Nginx já serve (ou ao seu próprio bloco de servidor numa porta própria), depois recarregue o Nginx.

    ```
    location /basic_status {
    stub_status;
    }
    ```

2. Restrinja essa localização ao anfitrião do DumbMonit, para que mais ninguém a possa ler; ajuste o endereço em conformidade.

    ```
    location /basic_status {
    stub_status;
    allow 10.0.0.5;
    deny all;
    }
    nginx -s reload
    ```

3. Verifique que responde antes de configurar o DumbMonit.

    ```
    curl -s http://nginx.lan/basic_status
    ```

4. No DumbMonit, introduza o anfitrião do Nginx, por exemplo «nginx.lan», e o caminho se não for «/basic_status». Se o NGINX Plus estiver instalado, marque «Read the NGINX Plus API» para adicionar a saúde dos upstreams e os contadores por zona.

!!! warning
    O stub_status só conta ligações e pedidos: um erro de configuração num
    bloco de servidor, ou um backend que o nginx não consegue alcançar, não
    aparece aqui. A versão open-source não tem noção de «upstream»: isso só
    existe no NGINX Plus, lido através da sua própria API quando ativado.

O exemplo acima restringe a localização por IP, a configuração mais simples
para um homelab; um bloco `basic_auth` funciona igualmente bem se a página de
estado não puder estar atrás de uma regra de firewall própria. O próprio
`stub_status` não pode ser protegido por senha diretamente pelo Nginx — a
senha, se existir, pertence ao bloco `location` em redor.

## Credenciais {#credentials}

| Credencial | Campos |
|---|---|
| Sem senha | Nada: a página de estado é filtrada para que só o DumbMonit a alcance. |
| Autenticação básica | O utilizador e a senha do bloco que protege a página de estado, se existir. |

Endereço: um nome de anfitrião ou IP, com a porta se não for 80. Opções:
protocolo (HTTP por predefinição), porta, caminho de estado (`/basic_status`
por predefinição), verificação de certificado, tempo limite do pedido (10 s),
e o interruptor e a versão da API do NGINX Plus.
