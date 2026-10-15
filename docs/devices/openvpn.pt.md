# Servidor OpenVPN {#openvpn-server}

O processo servidor do OpenVPN expõe o seu próprio canal de controlo, a
**interface de gestão**: um serviço TCP local, separado da própria VPN, feito
para uma ferramenta de monitorização e não para um cliente VPN. O DumbMonit lê-a
como qualquer cliente de gestão: a versão do servidor, se se declara ligado, e
cada cliente atualmente em sessão, com o seu endereço real e virtual e o seu
tráfego. Nunca é enviado nada que altere o estado do servidor ou desligue um
cliente.

Os clientes são transitórios por natureza — um portátil que entra em suspensão
desaparece simplesmente da leitura seguinte. O DumbMonit não tenta lembrar-se de
quem estava ligado há cinco minutos; reporta apenas quem está ligado agora, mais
o número de clientes como métrica que vale a pena representar ao longo do tempo.

## O que monitoriza {#what-it-watches}

Todas as métricas têm o prefixo `dumbmonit_openvpn_`.

| Métrica | O quê | Etiquetas |
|---|---|---|
| `server_info` | valor 1 | `version` |
| `server_up` | 1 quando a interface de gestão reporta o servidor como `CONNECTED`, 0 caso contrário | `state` |
| `server_state_age_seconds` | tempo desde que o servidor entrou no seu estado atual | |
| `clients_connected` | número de clientes atualmente em sessão | |
| `client_bytes_received`, `client_bytes_sent` | tráfego de um cliente (contadores) | `common_name`, `real_address` |
| `client_connected_seconds` | tempo desde o início da sessão desse cliente | `common_name`, `real_address` |

`real_address` nunca inclui a porta de origem do cliente: muda a cada
reconexão atrás de NAT e multiplicaria as séries sem necessidade. No máximo 256
clientes têm cada um uma série detalhada; `clients_connected` continua exato
para além disso.

Regra incorporada que se aplica: **Monitor unreachable**, quando a própria
ligação de gestão não pode ser estabelecida ou a palavra-passe é recusada.

## A página do monitor {#the-device-page}

O painel acima dos gráficos lê o que a verificação guardou; abrir a página nunca
se liga à interface de gestão. Mostra a versão do servidor, se se declara
ligado, o número de clientes, e uma lista dos clientes atualmente em sessão com
o seu endereço e tráfego.

## Ativar a interface de gestão {#turn-on-the-management-interface}

1. A interface de gestão é um serviço separado, desativado por predefinição: adicione isto à configuração do servidor (server.conf, ou o as.conf do OpenVPN Access Server, sob outro nome) e reinicie-o. Associe-o a um endereço que o anfitrião do DumbMonit consiga alcançar, ou a 127.0.0.1 e alcance-o através de um túnel SSH ou de um agente de retransmissão na mesma máquina.

    ```
    management 0.0.0.0 7505
    ```

2. Opcional mas recomendado: a interface de gestão não tem conta própria, apenas uma palavra-passe partilhada. Coloque-a num ficheiro legível apenas pelo processo OpenVPN e adicione-a como terceiro argumento.

    ```
    management 0.0.0.0 7505 /etc/openvpn/management-password
    ```

3. No DumbMonit, introduza o endereço do servidor, por exemplo «vpn.lan» (porta 7505), e a palavra-passe de gestão, se estiver definida.

4. O DumbMonit apenas lê: envia version, state e status 3, nunca signal, kill ou client-kill. Não pode desligar um cliente, alterar uma definição nem parar o servidor.

!!! warning
    Quem alcançar a porta de gestão e conhecer a palavra-passe pode desligar clientes, ler todos os seus endereços IP reais e reconfigurar o servidor em execução: mantenha-a fora da rede pública, atrás de uma regra de firewall que permita apenas o anfitrião do DumbMonit, e com uma palavra-passe definida.

## Credenciais {#credentials}

| Credencial | Campos |
|---|---|
| Sem palavra-passe | Apenas para uma interface de gestão iniciada sem argumento de ficheiro de palavra-passe. |
| Palavra-passe de gestão | A palavra-passe partilhada definida pelo terceiro argumento da diretiva «management» do servidor. |

Endereço: um nome de anfitrião ou IP, opcionalmente com `:porta`; a porta
predefinida é 7505, uma convenção da documentação do próprio OpenVPN, não um
padrão imposto pelo servidor. Opções: porta (7505) e o tempo limite por troca
(10 s).

Uma ligação recusada ou sem resposta conta como servidor inacessível. Uma
palavra-passe errada é mostrada como erro de credenciais; nenhuma das duas
acorda ninguém de noite. A ligação é TCP simples: nada do que é lido é secreto,
mas a palavra-passe, se existir, atravessa a rede em claro a cada verificação —
alcance a interface apenas a partir de uma rede de confiança ou através de um
agente de retransmissão na mesma máquina que o servidor.
