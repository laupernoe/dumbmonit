# Nginx Proxy Manager {#nginx-proxy-manager}

O gestor de proxy: anfitriões cuja configuração o nginx recusou, anfitriões
desativados, e os certificados que os anfitriões apresentam, antes de uma falha
de renovação do Let's Encrypt se transformar num site com o certificado
expirado.

O DumbMonit inicia sessão na API REST do Nginx Proxy Manager (`/api`, na porta
da sua interface web, 81) com um utilizador próprio, depois lê os anfitriões
proxy, os anfitriões de redirecionamento, os anfitriões 404 (chamados dead
hosts na API) e os streams. Cada anfitrião indica se está ativado e se o nginx
aceitou a sua configuração. Um anfitrião que o nginx recusou não é servido, e a
interface web só o mostra como um ponto vermelho.

Os certificados não são lidos a partir da API. `/api/nginx/certificates`
entrega a chave privada dos certificados importados, e as credenciais do
desafio DNS, a qualquer utilizador autorizado a ver certificados. Em vez disso,
o DumbMonit lê cada certificado onde é servido: um handshake TLS na porta 443
do Nginx Proxy Manager, com o nome de um anfitrião que o usa. Isto também mede
o certificado que os visitantes realmente recebem.

## O que monitoriza {#what-it-watches}

Todas as métricas têm o prefixo `dumbmonit_npm_`.

| Métrica | O quê | Etiquetas |
|---|---|---|
| `version_info` | valor 1 | `version` |
| `hosts`, `hosts_enabled`, `hosts_disabled` | anfitriões de cada tipo | `type`: `proxy`, `redirection`, `dead` (anfitriões 404), `stream` |
| `hosts_offline` | anfitriões ativados cuja configuração o nginx recusou | `type` |
| `host_online` | por anfitrião ativado: 0 quando o nginx recusou a sua configuração | `type`, `proxy_host` |
| `host_error_info` | valor 1, para um anfitrião que o nginx recusou: a primeira linha do erro do nginx | `type`, `proxy_host`, `error` |
| `certificates_checked`, `certificates_read` | certificados associados a um anfitrião ativado, e os lidos na porta HTTPS | |
| `cert_expiry_days` | dias que faltam para expirar o certificado que um anfitrião apresenta | `proxy_host` |

`proxy_host` é o primeiro nome de domínio do anfitrião, ou `port 9000` para um
stream. Os anfitriões desativados são contados, não descritos: desativar um
anfitrião é deliberado, e o seu alerta, se existir, resolve-se. Os anfitriões
são descritos um a um até 500, e são lidos no máximo 100 certificados por
verificação.

As [regras incorporadas](../alerting/rules.md#reverse-proxies-and-domains) que
se aplicam: Nginx Proxy Manager host offline, Proxy certificate not renewed,
mais Device unreachable.

## A página do dispositivo {#the-device-page}

O painel acima dos gráficos lê o que a sonda guardou; abrir a página nunca
consulta o Nginx Proxy Manager. Diz numa frase o que está errado (anfitriões
que o nginx recusou, com o motivo, certificados perto de expirar, certificados
que não puderam ser lidos), depois os anfitriões proxy, os redirecionamentos,
os anfitriões 404, os streams e os anfitriões desativados, depois os
certificados, os que expiram primeiro à cabeça.

## Criar um utilizador do Nginx Proxy Manager só de visualização {#create-a-view-only-nginx-proxy-manager-user}

1. No Nginx Proxy Manager, abra Users e adicione um utilizador para o DumbMonit, com um endereço de email próprio como dumbmonit@example.com. Deixe todos os papéis por marcar.

2. No menu desse utilizador, abra Permissions: Item Visibility All Items; Proxy Hosts, Redirection Hosts, 404 Hosts e Streams View Only; Access Lists e SSL Certificates Hidden. Depois defina a sua palavra-passe com Change Password no mesmo menu.

3. SSL Certificates fica Hidden de propósito: o Nginx Proxy Manager entrega a chave privada dos certificados importados, e as credenciais do desafio DNS, a qualquer utilizador autorizado a vê-los. Em vez disso, o DumbMonit lê cada certificado onde é servido, com um handshake TLS na porta 443 para cada anfitrião que tenha um.

4. No DumbMonit, introduza o anfitrião do Nginx Proxy Manager com a porta da sua interface web, por exemplo «npm.lan:81», e o email e a palavra-passe desse utilizador. Se os anfitriões são servidos por HTTPS numa porta diferente da 443, defina a porta HTTPS abaixo.

!!! warning
    Um anfitrião cuja configuração o nginx recusa (um erro de escrita em Advanced, um ficheiro de certificado em falta) não é servido, e o Nginx Proxy Manager só o mostra como um ponto vermelho na lista. Os certificados do Let's Encrypt são renovados trinta dias antes: um que expira em menos de catorze dias é um cuja renovação falhou, e o DumbMonit avisa nesse caso.

Item Visibility All Items é importante: com Created Items, o utilizador só vê os
anfitriões que ele próprio criou, que são nenhum. Um utilizador sem Proxy Hosts
View Only é recusado; um sem Streams, Redirection Hosts ou 404 Hosts
simplesmente não os reporta.

O DumbMonit guarda o token que o Nginx Proxy Manager dá no início de sessão
(válido um dia) e volta a iniciar sessão quando expira, em vez de o fazer a
cada verificação.

## Credenciais {#credentials}

| Credencial | Campos |
|---|---|
| Email e palavra-passe do utilizador | O endereço de email e a palavra-passe do utilizador criado acima. |

Endereço: um nome de anfitrião ou IP com a porta da interface web
(`npm.lan:81`), ou um URL completo. Opções: protocolo (HTTP por predefinição),
porta da interface web (81), Verificar os certificados (ativado), porta HTTPS
(443), verificação do certificado da API e tempo limite do pedido (10 s).

A verificação de certificados liga-se à parte do anfitrião do endereço: quando
a interface web é alcançada através de outro proxy, aponte o endereço para o
próprio Nginx Proxy Manager, ou desmarque Verificar os certificados e vigie
cada site com uma [verificação de certificado TLS](services.md). Os
certificados wildcard são lidos através do primeiro anfitrião que os usa.
