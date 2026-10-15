# XCP-ng / XenServer {#xcp-ng-xenserver}

Um pool XCP-ng ou XenServer (Citrix Hypervisor), lido através do XAPI, a API
de gestão que cada anfitrião já executa. Não é preciso o Xen Orchestra.

A cada minuto, o DumbMonit envia chamadas JSON-RPC para
`https://<coordenador>/jsonrpc`: um `session.login_with_password` (a sessão é
depois mantida de uma consulta para a seguinte e reaberta uma vez se o pool a
invalidar) e um `get_all_records` por classe: `host`, `host_metrics`, `VM`,
`VM_metrics`, `SR` e `pool`. Nada é alguma vez alterado: a conta tem a função
integrada `read-only`, que o XAPI aplica a cada chamada.

Adicione o pool uma só vez, pelo endereço do seu coordenador: este reporta
todos os anfitriões, VM e repositórios de armazenamento. Se o endereço que
introduziu for o de um membro do pool, este responde `HOST_IS_SLAVE` com o
endereço do coordenador; o DumbMonit segue-o uma vez, com o mesmo protocolo e
porta, pelo que uma mudança de coordenador não o obriga a editar o
monitor.

!!! warning "Validado apenas contra a documentação do fornecedor — ainda não testado num sistema real"

    Esta integração foi construída a partir da documentação do XAPI (o
    protocolo JSON-RPC, as classes `host`, `VM`, `SR` e `pool`) e testada
    contra respostas gravadas, não contra um pool em funcionamento. Diga-nos o
    que falha.

## O que monitoriza {#what-it-watches}

Todas as métricas têm o prefixo `dumbmonit_xcpng_`.

| Métrica | Etiquetas | Significado |
|---|---|---|
| `hosts`, `hosts_enabled` | | Anfitriões do pool, e quantos aceitam novas VM. |
| `host_info` | `host`, `version` | Valor 1: a versão do produto de cada anfitrião. |
| `host_enabled` | `host` | 0 quando o anfitrião está desativado (manutenção ou evacuado). |
| `host_live` | `host` | 1 quando o heartbeat do anfitrião chega ao coordenador, 0 quando está offline. |
| `host_cpu_cores`, `host_memory_total_bytes`, `host_memory_used_bytes` | `host` | Capacidade e memória em uso. |
| `vms`, `vms_running` | | VM, sem modelos, instantâneos nem domínios de controlo. |
| `vm_power_state_info` | `vm`, `state` | Valor 1: `running`, `halted`, `paused` ou `suspended`. |
| `vm_running` | `vm` | 1 quando a VM está em execução. |
| `vm_vcpus`, `vm_memory_bytes` | `vm` | vCPU no arranque; memória em uso para uma VM em execução, o seu máximo configurado caso contrário. |
| `vm_ha_protected_down` | `vm` | Apenas para VM com prioridade de reinício, num pool com alta disponibilidade ativa: 1 quando a VM está parada. |
| `srs` | | Repositórios de armazenamento. |
| `sr_size_bytes`, `sr_used_bytes`, `sr_used_percent` | `sr` | Nível de ocupação (as bibliotecas ISO são contadas, não detalhadas). |
| `sr_shared` | `sr` | 1 para um repositório partilhado (NFS, iSCSI…). |
| `pool_info` | `pool`, `master` | Valor 1: o pool e o seu coordenador atual. |
| `pool_ha_enabled` | `pool` | 1 quando a alta disponibilidade está ativa. |
| `pool_ha_overcommitted` | `pool` | Com a HA ativa: 1 quando o pool já não consegue reiniciar as suas VM protegidas após as falhas de anfitrião que está configurado para tolerar. |
| `pool_ha_host_failures_to_tolerate` | `pool` | O número de falhas de anfitrião que a HA está configurada para suportar. |

O XenAPI não tem um estado de energia «em falha». Uma VM protegida encontrada
parada é o sinal mais próximo que a API dá de uma VM que parou contra a sua
vontade; se desligar uma VM protegida de propósito, retire primeiro a sua
prioridade de reinício.

As [regras incorporadas](../alerting/rules.md#xcp-ng-xenserver) que se aplicam:

- **XCP-ng host offline**: o heartbeat de um anfitrião não chega ao
  coordenador há cinco minutos (Warning).
- **XCP-ng SR almost full**: um repositório de armazenamento está 90% cheio
  durante quinze minutos (Advisory).
- **XCP-ng HA-protected VM down**: uma VM protegida pela alta disponibilidade
  está parada há dois minutos (Warning).
- **XCP-ng pool HA overcommitted**: o pool está sobrecomprometido há cinco
  minutos (Advisory).

## Dar ao DumbMonit uma conta XAPI só de leitura {#give-dumbmonit-a-read-only-xapi-account}

1. O XAPI só aceita contas diferentes do seu superutilizador integrado quando o pool usa autenticação externa. Se o pool já estiver associado ao Active Directory, crie lá uma conta dumbmonit e passe ao passo 3.

2. Caso contrário, crie uma conta local com uma senha longa e aleatória em cada anfitrião do pool e depois ative uma vez a autenticação PAM, a partir do coordenador do pool. Enquanto não lhe for dada uma função, a conta não pode chamar nada.

    ```
    useradd -M -s /sbin/nologin dumbmonit
    passwd dumbmonit
    xe pool-enable-external-auth auth-type=PAM service-name=pam
    ```

3. No coordenador do pool, registe a conta como sujeito e depois dê a esse sujeito a função integrada read-only, com o UUID que o primeiro comando imprime.

    ```
    xe subject-add subject-name=dumbmonit
    xe subject-role-add uuid=SUBJECT_UUID role-name=read-only
    ```

4. No DumbMonit, introduza o endereço do coordenador do pool, por exemplo «xcp01.lan», com esse nome de utilizador e senha. Adicione o pool uma só vez: o coordenador reporta todos os anfitriões, VM e repositórios de armazenamento. Um membro do pool responde com o endereço do coordenador, que o DumbMonit segue depois.

5. O DumbMonit só lê, através da interface JSON-RPC do XAPI (/jsonrpc): nunca inicia, para nem migra uma VM e nunca altera um repositório de armazenamento nem a alta disponibilidade do pool.

!!! warning
    Nunca dê ao DumbMonit a senha de superutilizador dos anfitriões: com a função read-only, uma senha divulgada não pode alterar nada. O XCP-ng e o XenServer trazem um certificado autoassinado: marque Accept an unverifiable certificate a menos que o anfitrião do DumbMonit confie no certificado que o pool apresenta.

## Credenciais {#credentials}

| Credencial | Campos |
|---|---|
| Nome de utilizador e senha | A conta `dumbmonit`, registada como sujeito com a função `read-only`. |

Endereço: o nome do anfitrião ou o IP do coordenador do pool, `host:port`, ou
um URL. Opções:

- **Porta** (443).
- **Aceitar um certificado não verificável**: o próprio certificado dos
  anfitriões é autoassinado.
- **Tempo limite por pedido** (15 s).

## Resolução de problemas {#troubleshooting}

«XCP-ng refused the user name or password»: senha errada, ou a conta não está
registada como sujeito (`xe subject-list`).

«XCP-ng accepted the account but refused the call»: o sujeito não tem função,
ou tem uma função sem acesso de leitura; dê-lhe `read-only`.

«This host is a pool member, not the coordinator»: o DumbMonit não conseguiu
alcançar o coordenador que o membro indicou; introduza você mesmo o endereço
do coordenador.
