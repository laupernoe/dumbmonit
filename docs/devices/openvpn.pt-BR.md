# Servidor OpenVPN {#openvpn-server}

O processo servidor do OpenVPN expõe seu próprio canal de controle, a
**interface de gerenciamento**: um listener TCP local, separado da VPN em si,
feito para uma ferramenta de monitoramento e não para um cliente VPN. O
DumbMonit a lê como qualquer cliente de gerenciamento: a versão do servidor,
se ele se declara conectado e cada cliente atualmente em sessão, com seu
endereço real e virtual e seu tráfego. Nada é enviado que mude o estado do
servidor ou desconecte um cliente.

Os clientes são transitórios por natureza: um laptop que entra em suspensão
simplesmente desaparece da leitura seguinte. O DumbMonit não tenta lembrar
quem estava conectado cinco minutos atrás; ele reporta apenas quem está
conectado agora, mais o número de clientes como uma métrica que vale a pena
acompanhar em gráfico ao longo do tempo.

## O que ele observa {#what-it-watches}

Todas as métricas têm o prefixo `dumbmonit_openvpn_`.

| Métrica | O quê | Rótulos |
|---|---|---|
| `server_info` | valor 1 | `version` |
| `server_up` | 1 quando a interface de gerenciamento reporta o servidor como `CONNECTED`, 0 caso contrário | `state` |
| `server_state_age_seconds` | tempo desde que o servidor entrou em seu estado atual | |
| `clients_connected` | número de clientes atualmente em sessão | |
| `client_bytes_received`, `client_bytes_sent` | tráfego de um cliente (contadores) | `common_name`, `real_address` |
| `client_connected_seconds` | tempo desde o início da sessão desse cliente | `common_name`, `real_address` |

`real_address` nunca inclui a porta de origem do cliente: ela muda a cada
reconexão atrás de um NAT e multiplicaria as séries à toa. No máximo 256
clientes recebem cada um uma série detalhada; `clients_connected` continua
exato além disso.

Regras integradas que se aplicam: **Monitor unreachable**, quando a própria
conexão de gerenciamento não pode ser feita ou a senha é recusada.

## A página do monitor {#the-device-page}

O painel acima dos gráficos lê o que a sonda armazenou; abrir a página nunca
conecta à interface de gerenciamento. Ele mostra a versão do servidor e se
ele se declara conectado, o número de clientes e uma lista dos clientes
atualmente em sessão, com seu endereço e tráfego.

## Ative a interface de gerenciamento {#turn-on-the-management-interface}

1. A interface de gerenciamento é um listener separado, desativado por padrão: adicione isto à configuração do servidor (server.conf, ou o as.conf do OpenVPN Access Server, que tem outro nome) e reinicie-o. Associe-a a um endereço que o host do DumbMonit consiga alcançar, ou a 127.0.0.1 e alcance-a por um túnel SSH ou por um agente relay na mesma máquina.

    ```
    management 0.0.0.0 7505
    ```

2. Opcional, mas recomendado: a interface de gerenciamento não tem conta própria, apenas uma senha compartilhada. Coloque uma em um arquivo legível somente pelo processo OpenVPN e adicione-a como terceiro argumento.

    ```
    management 0.0.0.0 7505 /etc/openvpn/management-password
    ```

3. No DumbMonit, informe o endereço do servidor, por exemplo "vpn.lan" (porta 7505), e a senha de gerenciamento, se houver uma.

4. O DumbMonit apenas lê: ele envia version, state e status 3, nunca signal, kill ou client-kill. Ele não pode desconectar um cliente, mudar uma configuração nem parar o servidor.

!!! warning
    Qualquer pessoa que alcance a porta de gerenciamento e conheça a senha pode desconectar clientes, ler todos os seus endereços IP reais e reconfigurar o servidor em execução: mantenha-a fora da rede pública, atrás de uma regra de firewall que permita apenas o host do DumbMonit, com uma senha definida.

## Credenciais {#credentials}

| Credencial | Campos |
|---|---|
| Sem senha | Apenas para uma interface de gerenciamento iniciada sem o argumento de arquivo de senha. |
| Senha de gerenciamento | A senha compartilhada definida pelo terceiro argumento da diretiva "management" do servidor. |

Endereço: um nome de host ou IP, opcionalmente com `:porta`; a porta padrão é
7505, uma convenção da própria documentação do OpenVPN, não um padrão que o
servidor imponha. Opções: porta (7505) e o timeout por troca (10 s).

Uma conexão recusada ou a falta de resposta conta como servidor inacessível.
Uma senha errada é exibida como erro de credencial; nenhum dos dois acorda
ninguém à noite. A conexão é TCP simples: nada do que é lido é secreto, mas a
senha, se houver uma, atravessa a rede em texto claro a cada verificação:
alcance a interface apenas a partir de uma rede confiável ou por um agente
relay na mesma máquina que o servidor.
