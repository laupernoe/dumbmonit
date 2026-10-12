# Sophos Firewall

Um Sophos Firewall (SFOS) através de sua API XML: o link de cada interface
vinculada a uma zona, e quais conexões IPsec estão ativadas.

Isso é deliberadamente tudo. A API XML é uma API de configuração: além do
status de conectividade somente leitura de cada interface, ela não expõe
nenhum estado em tempo real. `<Get><IPSecConnection>` retorna se um
administrador ativou uma conexão, não se o túnel está ativo; alta
disponibilidade, licenças e assinaturas e a versão do firmware não estão nela
de forma alguma. O DumbMonit publica a ativação como tal e nunca a apresenta
como estado de túnel.

A cada minuto o DumbMonit envia duas requisições para
`https://<firewall>:4444/webconsole/APIController` (campo de formulário
`reqxml`, nunca na URL, onde a senha acabaria nos logs): `<Get><Interface>` e
`<Get><IPSecConnection>`. Cada uma carrega o nome de usuário e senha de uma
conta cujo perfil de acesso ao dispositivo é somente leitura em todo lugar: o
firewall aplica esse perfil também na API.

!!! warning "Validado apenas contra a documentação do fornecedor — ainda não testado em um sistema real"

    Esta integração foi construída a partir da documentação da API XML da
    Sophos (as entidades `Interface` e `IPSecConnection`, as mensagens de
    login e status), não contra um firewall em execução. O texto exato do
    status da interface (`Connected, 1000 Mbps…` ou `Disconnected`) vem
    dessa documentação e relatos da comunidade. Avise-nos o que não funcionar.

## O que ele observa

Todas as métricas têm o prefixo `dumbmonit_sophos_`.

| Métrica | Rótulos | Significado |
|---|---|---|
| `api_version_info` | `api_version` | Valor 1: a versão da API que o firewall anuncia (`2000.2`), que acompanha a versão do SFOS. |
| `interface_link_up` | `interface`, `zone` | 1 quando o status mostra Connected, 0 quando Disconnected; ausente quando o status é outra coisa. |
| `interface_enabled` | `interface`, `zone` | 0 quando a interface está desligada. |
| `interface_down` | `interface`, `zone` | 1 quando a interface está ligada, vinculada a uma zona, e não tem link. Portas livres (zona None) são deixadas de lado. |
| `interfaces`, `interfaces_down` | | Contagens. |
| `ipsec_connection_activated` | `connection` | 1 quando a conexão está ativada na configuração. Não é o estado do túnel. |
| `ipsec_connections` | | Contagem. |

A [regra integrada](../alerting/rules.md#sophos-firewall) que se aplica:

- **Sophos interface without link**: uma interface ligada e vinculada a uma
  zona não teve link por cinco minutos (Advisory).

Para contadores de tráfego, adicione o firewall também como um
[dispositivo SNMP](snmp.md): o SFOS responde à MIB de interface padrão.

## A página do dispositivo

O painel Firewall diz se toda interface em uma zona tem seu link, depois
lista essas interfaces e as conexões IPsec com sua ativação configurada,
rotulada como tal.

## Permitir que o DumbMonit leia a API XML do Sophos Firewall

1. Em Profiles > Device access, adicione um perfil chamado DumbMonit
   read-only e defina toda permissão como Read-only.

2. Em Authentication > Users, adicione um usuário chamado dumbmonit, do tipo
   Administrator, com esse perfil e uma senha longa e aleatória.

3. Em Backup & firmware > API (Administration > API access a partir do
   SFOS 22), ative a configuração da API e adicione o endereço do servidor
   DumbMonit aos endereços IP permitidos.

4. No DumbMonit, informe o endereço do firewall, por exemplo "sophos.lan",
   com o nome de usuário e senha do usuário dumbmonit. A API escuta na porta
   de administração, 4444 por padrão, com um certificado autoassinado:
   marque Accept an unverifiable certificate a menos que você tenha
   instalado o seu próprio.

!!! warning
    A API XML é uma API de configuração: ela reporta o link de cada
    interface e se cada conexão IPsec está ativada, não se o túnel está
    ativo. Alta disponibilidade, licenças e a versão do firmware não estão
    nela.

## Credenciais

| Credencial | Campos |
|---|---|
| Nome de usuário e senha | O usuário `dumbmonit` com o perfil de acesso ao dispositivo somente leitura. |

Endereço: o nome de host ou IP do firewall, `host:porta`, ou uma URL. Opções:

- **Porta** (4444): a porta de administração.
- **Aceitar um certificado não verificável**: o próprio certificado do
  firewall é autoassinado.
- **Tempo limite por requisição** (15 s).

## Solução de problemas

"Sophos Firewall refused the request (534 …)": o endereço do DumbMonit não
está entre os endereços IP permitidos na configuração da API.

"Sophos Firewall refused the login": nome de usuário ou senha errados, ou
uma senha com caracteres que o firewall rejeita em chamadas de API; uma senha
longa de letras e dígitos evita o problema.

"Sophos Firewall did not answer in XML": porta errada, ou a API está
desligada.
