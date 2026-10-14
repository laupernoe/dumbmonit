# XCP-ng / XenServer {#xcp-ng-xenserver}

Um pool XCP-ng ou XenServer (Citrix Hypervisor), lido pelo XAPI, a API de
gerenciamento que cada host já executa. Não é preciso o Xen Orchestra.

A cada minuto, o DumbMonit envia chamadas JSON-RPC para
`https://<coordenador>/jsonrpc`: um `session.login_with_password` (a sessão é
mantida de uma consulta para a seguinte e reaberta uma vez se o pool a
invalidar) e um `get_all_records` por classe: `host`, `host_metrics`, `VM`,
`VM_metrics`, `SR` e `pool`. Nada nunca é alterado: a conta tem o papel
integrado `read-only`, que o XAPI aplica a cada chamada.

Adicione o pool uma única vez, pelo endereço do seu coordenador: ele reporta
todos os hosts, VMs e repositórios de armazenamento. Se o endereço que você
informou for o de um membro do pool, ele responde `HOST_IS_SLAVE` com o
endereço do coordenador; o DumbMonit o segue uma vez, com o mesmo protocolo e
porta, de modo que uma troca de coordenador não obriga você a editar o
dispositivo.

!!! warning "Validado apenas contra a documentação do fornecedor — ainda não testado em um sistema real"

    Esta integração foi construída a partir da documentação do XAPI (o
    protocolo JSON-RPC, as classes `host`, `VM`, `SR` e `pool`) e testada
    contra respostas gravadas, não contra um pool em execução. Avise-nos o que
    não funcionar.

## O que ele observa {#what-it-watches}

Todas as métricas têm o prefixo `dumbmonit_xcpng_`.

| Métrica | Etiquetas | Significado |
|---|---|---|
| `hosts`, `hosts_enabled` | | Hosts do pool, e quantos aceitam novas VMs. |
| `host_info` | `host`, `version` | Valor 1: a versão do produto de cada host. |
| `host_enabled` | `host` | 0 quando o host está desabilitado (manutenção ou evacuado). |
| `host_live` | `host` | 1 quando o heartbeat do host chega ao coordenador, 0 quando está offline. |
| `host_cpu_cores`, `host_memory_total_bytes`, `host_memory_used_bytes` | `host` | Capacidade e memória em uso. |
| `vms`, `vms_running` | | VMs, sem templates, snapshots nem domínios de controle. |
| `vm_power_state_info` | `vm`, `state` | Valor 1: `running`, `halted`, `paused` ou `suspended`. |
| `vm_running` | `vm` | 1 quando a VM está em execução. |
| `vm_vcpus`, `vm_memory_bytes` | `vm` | vCPUs na inicialização; memória em uso para uma VM em execução, o máximo configurado caso contrário. |
| `vm_ha_protected_down` | `vm` | Apenas para VMs com prioridade de reinício, em um pool com alta disponibilidade ativada: 1 quando a VM está parada. |
| `srs` | | Repositórios de armazenamento. |
| `sr_size_bytes`, `sr_used_bytes`, `sr_used_percent` | `sr` | Nível de ocupação (as bibliotecas ISO são contadas, não detalhadas). |
| `sr_shared` | `sr` | 1 para um repositório compartilhado (NFS, iSCSI…). |
| `pool_info` | `pool`, `master` | Valor 1: o pool e seu coordenador atual. |
| `pool_ha_enabled` | `pool` | 1 quando a alta disponibilidade está ativada. |
| `pool_ha_overcommitted` | `pool` | Com a HA ativada: 1 quando o pool não consegue mais reiniciar suas VMs protegidas após as falhas de host que está configurado para tolerar. |
| `pool_ha_host_failures_to_tolerate` | `pool` | O número de falhas de host que a HA está configurada para suportar. |

O XenAPI não tem um estado de energia «travado». Uma VM protegida encontrada
parada é o sinal mais próximo que a API dá de uma VM que parou contra a sua
vontade; se você desligar uma VM protegida de propósito, remova antes a
prioridade de reinício dela.

As [regras integradas](../alerting/rules.md#xcp-ng-xenserver) que se aplicam:

- **XCP-ng host offline**: o heartbeat de um host não chega ao coordenador há
  cinco minutos (Alerta).
- **XCP-ng SR almost full**: um repositório de armazenamento está 90% cheio
  por quinze minutos (Aviso).
- **XCP-ng HA-protected VM down**: uma VM protegida pela alta disponibilidade
  está parada há dois minutos (Alerta).
- **XCP-ng pool HA overcommitted**: o pool está sobrecomprometido há cinco
  minutos (Aviso).

## Dar ao DumbMonit uma conta XAPI somente leitura {#give-dumbmonit-a-read-only-xapi-account}

1. O XAPI só aceita contas diferentes do seu superusuário integrado quando o pool usa autenticação externa. Se o pool já estiver associado ao Active Directory, crie lá uma conta dumbmonit e vá para o passo 3.

2. Caso contrário, crie uma conta local com uma senha longa e aleatória em cada host do pool e depois ative uma vez a autenticação PAM, a partir do coordenador do pool. Enquanto não receber um papel, a conta não pode chamar nada.

    ```
    useradd -M -s /sbin/nologin dumbmonit
    passwd dumbmonit
    xe pool-enable-external-auth auth-type=PAM service-name=pam
    ```

3. No coordenador do pool, registre a conta como sujeito e depois dê a esse sujeito o papel integrado read-only, com o UUID que o primeiro comando imprime.

    ```
    xe subject-add subject-name=dumbmonit
    xe subject-role-add uuid=SUBJECT_UUID role-name=read-only
    ```

4. No DumbMonit, informe o endereço do coordenador do pool, por exemplo "xcp01.lan", com esse nome de usuário e senha. Adicione o pool uma única vez: o coordenador reporta todos os hosts, VMs e repositórios de armazenamento. Um membro do pool responde com o endereço do coordenador, que o DumbMonit segue em seguida.

5. O DumbMonit só lê, pela interface JSON-RPC do XAPI (/jsonrpc): nunca inicia, para nem migra uma VM e nunca altera um repositório de armazenamento nem a alta disponibilidade do pool.

!!! warning
    Nunca dê ao DumbMonit a senha de superusuário dos hosts: com o papel read-only, uma senha vazada não pode alterar nada. O XCP-ng e o XenServer vêm com um certificado autoassinado: marque Accept an unverifiable certificate a menos que o host do DumbMonit confie no certificado que o pool apresenta.

## Credenciais {#credentials}

| Credencial | Campos |
|---|---|
| Nome de usuário e senha | A conta `dumbmonit`, registrada como sujeito com o papel `read-only`. |

Endereço: o nome do host ou o IP do coordenador do pool, `host:port`, ou uma
URL. Opções:

- **Porta** (443).
- **Aceitar um certificado não verificável**: o certificado próprio dos hosts
  é autoassinado.
- **Timeout por requisição** (15 s).

## Solução de problemas {#troubleshooting}

"XCP-ng refused the user name or password": senha errada, ou a conta não está
registrada como sujeito (`xe subject-list`).

"XCP-ng accepted the account but refused the call": o sujeito não tem papel,
ou tem um papel sem acesso de leitura; dê a ele `read-only`.

"This host is a pool member, not the coordinator": o DumbMonit não conseguiu
alcançar o coordenador que o membro indicou; informe você mesmo o endereço do
coordenador.
