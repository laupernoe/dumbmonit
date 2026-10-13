# Nginx Proxy Manager {#nginx-proxy-manager}

O gerenciador de proxy: hosts cuja configuração o nginx recusou, hosts
desativados, e os certificados que os hosts apresentam, antes de uma falha de
renovação do Let's Encrypt se transformar em um site com certificado expirado.

O DumbMonit faz login na API REST do Nginx Proxy Manager (`/api`, na porta de
sua interface web, 81) com um usuário próprio, e então lê os proxy hosts,
hosts de redirecionamento, hosts 404 (chamados dead hosts na API) e streams.
Cada host informa se está ativado e se o nginx aceitou sua configuração. Um
host que o nginx recusou não é servido, e a interface web só o mostra como um
ponto vermelho.

Certificados não são lidos da API. `/api/nginx/certificates` entrega a chave
privada de certificados importados, e as credenciais do desafio DNS, a
qualquer usuário autorizado a ver certificados. O DumbMonit lê cada
certificado onde ele é servido, em vez disso: um handshake TLS na porta 443
do Nginx Proxy Manager, com o nome de um host que o usa. Isso também mede o
certificado que os visitantes realmente recebem.

## O que ele observa {#what-it-watches}

Todas as métricas têm o prefixo `dumbmonit_npm_`.

| Métrica | O que é | Rótulos |
|---|---|---|
| `version_info` | valor 1 | `version` |
| `hosts`, `hosts_enabled`, `hosts_disabled` | hosts de cada tipo | `type`: `proxy`, `redirection`, `dead` (hosts 404), `stream` |
| `hosts_offline` | hosts ativados cuja configuração o nginx recusou | `type` |
| `host_online` | por host ativado: 0 quando o nginx recusou sua configuração | `type`, `host` |
| `host_error_info` | valor 1, para um host que o nginx recusou: a primeira linha do erro do nginx | `type`, `host`, `error` |
| `certificates_checked`, `certificates_read` | certificados associados a um host ativado, e os lidos na porta HTTPS | |
| `cert_expiry_days` | dias restantes antes que o certificado que um host apresenta expire | `host` |

`host` é o primeiro nome de domínio do host, ou `port 9000` para um stream.
Hosts desativados são contados, não descritos: desativar um host é
deliberado, e seu alerta, se houver, é resolvido. Hosts são descritos um a um
até 500, e no máximo 100 certificados são lidos por verificação.

As [regras integradas](../alerting/rules.md#reverse-proxies-and-domains) que
se aplicam: Nginx Proxy Manager host offline, Proxy certificate not renewed,
além de Device unreachable.

## A página do dispositivo {#the-device-page}

O painel acima dos gráficos lê o que a sondagem armazenou; abrir a página
nunca consulta o Nginx Proxy Manager. Ele diz o que está errado em uma frase
(hosts que o nginx recusou, com seu motivo, certificados próximos da
expiração, certificados que não puderam ser lidos), depois os proxy hosts,
redirecionamentos, hosts 404, streams e hosts desativados, depois os
certificados que expiram primeiro.

## Criar um usuário somente visualização no Nginx Proxy Manager {#create-a-view-only-nginx-proxy-manager-user}

1. No Nginx Proxy Manager, abra Users e adicione um usuário para o DumbMonit,
   com um endereço de e-mail próprio como dumbmonit@example.com. Deixe todos
   os papéis desmarcados.

2. No menu desse usuário, abra Permissions: Item Visibility All Items; Proxy
   Hosts, Redirection Hosts, 404 Hosts e Streams View Only; Access Lists e
   SSL Certificates Hidden. Depois defina sua senha com Change Password no
   mesmo menu.

3. SSL Certificates fica Hidden de propósito: o Nginx Proxy Manager entrega a
   chave privada de certificados importados, e as credenciais do desafio
   DNS, a qualquer usuário autorizado a vê-los. O DumbMonit lê cada
   certificado onde ele é servido, em vez disso, com um handshake TLS na
   porta 443 para cada host que tem um.

4. No DumbMonit, informe o host do Nginx Proxy Manager com a porta de sua
   interface web, por exemplo "npm.lan:81", e o e-mail e senha desse usuário.
   Se os hosts são servidos via HTTPS em outra porta além de 443, defina a
   porta HTTPS abaixo.

!!! warning
    Um host cuja configuração o nginx recusa (um erro de digitação em
    Advanced, um arquivo de certificado faltando) não é servido, e o Nginx
    Proxy Manager só o mostra como um ponto vermelho na lista. Certificados
    Let's Encrypt são renovados trinta dias antes: um que expira em menos de
    quatorze dias é um cuja renovação falhou, e o DumbMonit avisa nesse caso.

Item Visibility All Items importa: com Created Items, o usuário só vê os
hosts que ele mesmo criou, que são nenhum. Um usuário sem Proxy Hosts View
Only é recusado; um sem Streams, Redirection Hosts ou 404 Hosts simplesmente
não os reporta.

O DumbMonit mantém o token que o Nginx Proxy Manager dá no login (válido por
um dia) e faz login novamente quando ele expira, em vez de a cada verificação.

## Credenciais {#credentials}

| Credencial | Campos |
|---|---|
| E-mail e senha do usuário | O endereço de e-mail e senha do usuário criado acima. |

Endereço: um nome de host ou IP com a porta da interface web (`npm.lan:81`),
ou uma URL completa. Opções: protocolo (HTTP por padrão), porta da interface
web (81), Verificar os certificados (ativado), porta HTTPS (443), verificação
de certificado da API e tempo limite da requisição (10 s).

A verificação de certificado se conecta à parte do host do endereço: quando a
interface web é alcançada através de outro proxy, aponte o endereço para o
próprio Nginx Proxy Manager, ou desmarque Verificar os certificados e observe
cada site com uma [verificação de certificado TLS](services.md) em vez disso.
Certificados wildcard são lidos através do primeiro host que os usa.
