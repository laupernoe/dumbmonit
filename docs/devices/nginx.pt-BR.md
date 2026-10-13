# Nginx {#nginx}

O servidor web e proxy reverso: conexões ativas e em espera, as taxas de
accept/handled/request, e a saúde dos upstreams quando a API comercial NGINX
Plus está disponível.

O DumbMonit lê o módulo `stub_status` embutido do Nginx, um punhado de
contadores em texto simples, sem necessidade de API ou de conta em uma rede
corretamente filtrada. Quando o NGINX Plus comercial está instalado, o
DumbMonit também pode ler sua API JSON para saúde de upstream e contadores
por zona.

## O que ele observa {#what-it-watches}

Todas as métricas têm o prefixo `dumbmonit_nginx_`. As métricas `_total` são
contadores: gráficos e regras as transformam em taxas.

| Métrica | O que é | Rótulos |
|---|---|---|
| `connections_active` | conexões atualmente abertas | |
| `connections_reading`, `connections_writing`, `connections_waiting` | conexões pelo que estão fazendo | |
| `accepts_total`, `handled_total` | conexões aceitas, e realmente tratadas | |
| `requests_total` | requisições servidas desde que o Nginx iniciou | |
| `version_info` | presença; o cabeçalho de resposta `Server`, quando o Nginx envia um | `server` |
| `plus_upstream_server_up` | 1 quando o NGINX Plus marca o servidor como ativo | `upstream`, `server` |
| `plus_upstream_server_active`, `plus_upstream_server_requests_total`, `plus_upstream_server_fails_total`, `plus_upstream_server_5xx_total` | contadores por servidor do NGINX Plus | `upstream`, `server` |
| `plus_zone_requests_total`, `plus_zone_5xx_total`, `plus_zone_received_bytes_total`, `plus_zone_sent_bytes_total` | contadores por zona do NGINX Plus | `zone` |

Uma lacuna entre `accepts_total` e `handled_total` significa conexões que o
Nginx descartou antes de poder servi-las (geralmente `worker_connections`
atingido). As métricas `plus_*` só aparecem quando "Ler a API do NGINX Plus"
está marcado e o NGINX Plus responde; a versão de código aberto não tem um
"upstream" para reportar apenas com `stub_status`.

## A página do dispositivo {#the-device-page}

O painel acima dos gráficos lê o que a sondagem armazenou; abrir a página
nunca consulta o Nginx. Ele mostra conexões ativas, lendo, escrevendo e em
espera, as taxas de accept/handled/request nas últimas 24 horas, e, com
NGINX Plus, a saúde de upstream e o tráfego por zona.

## Ativar o módulo stub_status do Nginx {#enable-nginxs-stub_status-module}

1. O Nginx já vem com o stub_status embutido: só precisa de um local que o
   ative. Adicione isto ao bloco de servidor que o Nginx já serve (ou a seu
   próprio bloco de servidor em uma porta própria), depois recarregue o
   Nginx.

    ```
    location /basic_status {
    stub_status;
    }
    ```

2. Restrinja esse local ao host DumbMonit, para que ninguém mais possa
   lê-lo; ajuste o endereço conforme necessário.

    ```
    location /basic_status {
    stub_status;
    allow 10.0.0.5;
    deny all;
    }
    nginx -s reload
    ```

3. Verifique que ele responde antes de configurar o DumbMonit.

    ```
    curl -s http://nginx.lan/basic_status
    ```

4. No DumbMonit, informe o host do Nginx, por exemplo "nginx.lan", e o
   caminho se não for "/basic_status". Se o NGINX Plus estiver instalado,
   marque "Ler a API do NGINX Plus" para adicionar saúde de upstream e
   contadores por zona.

!!! warning
    O stub_status só conta conexões e requisições: um erro de configuração
    em um bloco de servidor, ou um backend que o nginx não consegue
    alcançar, não aparece aqui. A versão de código aberto não tem noção de
    "upstream": isso só existe no NGINX Plus, lido através de sua própria
    API quando ativado.

O exemplo acima restringe o local por IP, a configuração mais simples para um
homelab; um bloco `basic_auth` funciona igualmente bem se a página de status
não puder ficar detrás de uma regra de firewall própria. O `stub_status` em
si não pode ser protegido por senha diretamente pelo Nginx — a senha, se
houver, pertence ao bloco `location` ao redor.

## Credenciais {#credentials}

| Credencial | Campos |
|---|---|
| Sem senha | Nada: a página de status é filtrada para que só o DumbMonit a alcance. |
| Autenticação básica | O usuário e senha do bloco que protege a página de status, se houver. |

Endereço: um nome de host ou IP, com a porta se não for 80. Opções:
protocolo (HTTP por padrão), porta, caminho de status (`/basic_status` por
padrão), verificação de certificado, tempo limite da requisição (10 s), e o
seletor e versão da API do NGINX Plus.
