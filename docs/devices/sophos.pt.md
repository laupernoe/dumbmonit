# Sophos Firewall {#sophos-firewall}

Um Sophos Firewall (SFOS) através da sua API XML: a ligação de cada interface
associada a uma zona, e quais ligações IPsec estão ativadas.

Isso é, deliberadamente, tudo. A API XML é uma API de configuração: além do
estado de conectividade, só de leitura, de cada interface, não expõe nenhum
estado em tempo real. `<Get><IPSecConnection>` devolve se um administrador
ativou uma ligação, não se o túnel está ativo; a alta disponibilidade, as
licenças e subscrições e a versão do firmware não aparecem nela de forma
alguma. O DumbMonit publica a ativação como tal e nunca a apresenta como
estado de um túnel.

A cada minuto, o DumbMonit envia dois pedidos a
`https://<firewall>:4444/webconsole/APIController` (campo de formulário
`reqxml`, nunca no URL, onde a senha ficaria nos registos):
`<Get><Interface>` e `<Get><IPSecConnection>`. Cada um transporta o nome de
utilizador e a senha de uma conta cujo perfil de acesso ao dispositivo é só
de leitura em todo o lado: a firewall impõe esse perfil também na API.

!!! warning "Validado apenas contra a documentação do fornecedor — ainda não testado num sistema real"

    Esta integração foi construída a partir da documentação da API XML da
    Sophos (as entidades `Interface` e `IPSecConnection`, as mensagens de
    início de sessão e de estado), não contra uma firewall em funcionamento.
    O texto exato do estado da interface («Connected, 1000 Mbps…» ou
    «Disconnected») vem dessa documentação e de relatos da comunidade.
    Diga-nos o que falha.

## O que monitoriza {#what-it-watches}

Todas as métricas têm o prefixo `dumbmonit_sophos_`.

| Métrica | Etiquetas | Significado |
|---|---|---|
| `api_version_info` | `api_version` | Valor 1: a versão da API que a firewall anuncia (`2000.2`), que acompanha a versão do SFOS. |
| `interface_link_up` | `interface`, `zone` | 1 quando o estado lê Connected, 0 quando Disconnected; ausente quando o estado é outra coisa. |
| `interface_enabled` | `interface`, `zone` | 0 quando a interface está desligada. |
| `interface_down` | `interface`, `zone` | 1 quando a interface está ligada, associada a uma zona, e não tem ligação. As portas livres (zona None) são deixadas em paz. |
| `interfaces`, `interfaces_down` | | Contagens. |
| `ipsec_connection_activated` | `connection` | 1 quando a ligação está ativada na configuração. Não o estado do túnel. |
| `ipsec_connections` | | Contagem. |

A [regra incorporada](../alerting/rules.md#sophos-firewall) que se aplica:

- **Sophos interface without link**: uma interface ligada e associada a uma
  zona está sem ligação há cinco minutos (Advisory).

Para contadores de tráfego, adicione também a firewall como um [dispositivo
SNMP](snmp.md): o SFOS responde à MIB de interface padrão.

## A página do dispositivo {#the-device-page}

O painel Firewall diz se cada interface numa zona tem a sua ligação, depois
lista essas interfaces e as ligações IPsec com a sua ativação configurada,
etiquetada como tal.

## Permitir ao DumbMonit ler a API XML do Sophos Firewall {#allow-dumbmonit-to-read-the-sophos-firewall-xml-api}

1. Em Profiles > Device access, adicione um perfil chamado DumbMonit read-only e defina todas as permissões como Read-only.

2. Em Authentication > Users, adicione um utilizador chamado dumbmonit, do tipo Administrator, com esse perfil e uma senha longa e aleatória.

3. Em Backup & firmware > API (Administration > API access a partir do SFOS 22), ative a configuração da API e adicione o endereço do servidor DumbMonit aos endereços IP permitidos.

4. No DumbMonit, introduza o endereço da firewall, por exemplo «sophos.lan», com o nome de utilizador e a senha de dumbmonit. A API escuta na porta de administração, 4444 por predefinição, com um certificado autoassinado: marque Accept an unverifiable certificate a menos que tenha instalado o seu próprio.

!!! warning
    A API XML é uma API de configuração: reporta a ligação de cada interface e se cada ligação IPsec está ativada, não se o túnel está ativo. A alta disponibilidade, as licenças e a versão do firmware não aparecem nela.

## Credenciais {#credentials}

| Credencial | Campos |
|---|---|
| Nome de utilizador e senha | O utilizador `dumbmonit` com o perfil de acesso ao dispositivo só de leitura. |

Endereço: o nome de anfitrião ou IP da firewall, `anfitrião:porta`, ou um
URL. Opções:

- **Porta** (4444): a porta de administração.
- **Aceitar um certificado não verificável**: o próprio certificado da
  firewall é autoassinado.
- **Tempo limite por pedido** (15 s).

## Resolução de problemas {#troubleshooting}

«Sophos Firewall refused the request (534 …)»: o endereço do DumbMonit não
está entre os endereços IP permitidos da configuração da API.

«Sophos Firewall refused the login»: nome de utilizador ou senha errados, ou
uma senha com caracteres que a firewall rejeita nas chamadas de API; uma
senha longa de letras e dígitos evita a questão.

«Sophos Firewall did not answer in XML»: porta errada, ou a API está
desligada.
