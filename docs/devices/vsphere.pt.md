# VMware vSphere {#vmware-vsphere}

Um vCenter Server e os anfitriões que gere, ou um anfitrião ESXi autónomo:
anfitriões que deixam de responder, ficam em modo de manutenção ou reportam
saúde vermelha, máquinas virtuais e as suas VMware Tools, datastores a encher
ou a ficarem inacessíveis, e os alarmes que o próprio vSphere levantou.

O DumbMonit lê o vSphere com um utilizador dedicado que tem o papel incorporado
**Read-only**. Nunca liga nem desliga uma VM, nunca entra em modo de manutenção
e nunca reconhece um alarme.

A falha que vale a pena vigiar: **um datastore que enche**. Quando um datastore
com thin provisioning chega a 100%, todas as VM que precisam de escrever nele
são pausadas de uma vez. Os alarmes que o vCenter levanta também o dizem, mas
só a quem tem o cliente aberto.

## Porquê a Web Services API {#why-the-web-services-api}

O DumbMonit fala com a vSphere Web Services API (SOAP, `/sdk`), não com a API
REST (`/api/vcenter/*`): um anfitrião ESXi autónomo não tem API REST, e num
vCenter a API REST não indica o modo de manutenção, a saúde global
(`overallStatus`), o estado das VMware Tools nem os alarmes disparados. Uma
vista de todo o inventário é lida numa única passagem, página a página, e
libertada no fim.

A sessão é mantida de uma sonda para a seguinte e só reaberta quando expira: o
vCenter não regista um novo início de sessão a cada minuto.

## O que monitoriza {#what-it-watches}

Todas as métricas têm o prefixo `dumbmonit_vsphere_`. Os valores de saúde
seguem o `overallStatus` do vSphere: 0 verde, 1 amarelo, 2 vermelho, 3 cinzento
(desconhecido).

| Métrica | O quê | Etiquetas |
|---|---|---|
| `info` | valor 1 | `product` (`vcenter` ou `esxi`), `version`, `build` |
| `hosts` | anfitriões por estado de ligação: `connected`, `disconnected`, `notResponding` | `state` |
| `host_connection_state` | 0 ligado, 1 desligado por um administrador, 2 sem resposta | `esxi` |
| `host_power_state` | 0 ligado, 1 em espera, 2 desligado, 3 desconhecido | `esxi` |
| `host_maintenance` | 1 em modo de manutenção | `esxi` |
| `host_status` | saúde global | `esxi` |
| `host_cpu_percent`, `host_memory_percent`, `host_memory_bytes`, `host_uptime_seconds` | apenas anfitriões ligados | `esxi` |
| `vms` | VM por estado de energia: `poweredOn`, `poweredOff`, `suspended` (modelos excluídos) | `state` |
| `templates` | modelos de VM | |
| `vm_power_state` | 0 ligada, 1 desligada, 2 suspensa; no máximo 1000 VM | `vm` |
| `vm_status` | saúde global | `vm` |
| `vm_tools_status` | apenas VM ligadas: 0 atualizado, 1 versão antiga, 2 instalado mas não a correr, 3 não instalado | `vm` |
| `datastores` | datastores | |
| `datastore_capacity_bytes`, `datastore_free_bytes`, `datastore_used_percent` | | `datastore` |
| `datastore_accessible` | 1 acessível, 0 inacessível | `datastore` |
| `datastore_status` | saúde global | `datastore` |
| `alarms` | alarmes disparados não reconhecidos, por cor: `red`, `yellow` | `status` |
| `alarms_acknowledged` | alarmes disparados já reconhecidos | |
| `alarm` | uma série por alarme não reconhecido, valor = o seu código de saúde, no máximo 64 | `alarm` (o seu nome), `entity`, `entity_type`, `status` |
| `scrape_errors` | chamadas que falharam na última sonda | |

As [regras incorporadas](../alerting/rules.md#vmware-vsphere) que se aplicam:
vSphere host not responding, vSphere host health red, vSphere host left in
maintenance, vSphere datastore almost full, vSphere datastore inaccessible,
vSphere VM tools not running, vSphere red alarm, mais Device unreachable.

## A página do dispositivo {#the-device-page}

O painel acima dos gráficos lê o que a sonda guardou; abrir a página nunca
consulta o vSphere. Diz primeiro o que está errado (anfitriões sem resposta,
alarmes vermelhos, datastores quase cheios ou inacessíveis), depois os
anfitriões com a sua carga, os datastores com o seu nível de ocupação, e as VM
por estado de energia.

## Como foi validado {#how-it-was-validated}

Contra o simulador govmomi `vcsim` 0.56.0, em modo vCenter e em modo ESXi:
início de sessão e palavra-passe recusada, reutilização e expiração de sessão,
paginação do inventário, um anfitrião sem resposta, um anfitrião em modo de
manutenção, uma VM desligada e um modelo. O simulador não levanta nenhum
alarme: a leitura dos alarmes disparados é validada contra a estrutura
`AlarmState` documentada da Web Services API.

## Criar um utilizador só de leitura no vSphere {#create-a-read-only-user-in-vsphere}

1. Num vCenter: Administration → Single Sign On → Users and Groups → Users, domínio vsphere.local → Add. Dê ao utilizador o nome seguinte e uma palavra-passe longa e aleatória.

    ```
    dumbmonit
    ```

2. Ainda no vCenter: Administration → Access Control → Global Permissions → Add. Escolha esse utilizador, o papel Read-only, e marque Propagate to children. O Read-only vê todos os anfitriões, VM, datastores e alarmes, e não pode alterar nada.

3. Num anfitrião ESXi autónomo, em vez disso: no cliente do anfitrião, Manage → Security & users → Users → Add user, com o mesmo nome. Depois Host → Actions → Permissions → Add user: escolha-o, o papel Read-only, e marque Propagate to all children.

4. No DumbMonit, introduza o endereço do vCenter ou do anfitrião, por exemplo «vcenter.lan» ou «esxi1.lan», com o nome de utilizador tal como foi criado: dumbmonit@vsphere.local num vCenter, dumbmonit num anfitrião. Quando um vCenter gere os anfitriões, adicione apenas o vCenter: ele reporta todos os anfitriões, VM e datastores.

5. O DumbMonit só lê, através da vSphere Web Services API (/sdk). Nunca liga nem desliga uma VM, nunca entra em modo de manutenção e nunca reconhece um alarme.

!!! warning
    Não reutilize a conta do domínio de início de sessão único nem a conta incorporada do anfitrião: o papel Read-only acima é tudo o que o DumbMonit precisa. O vCenter e o ESXi usam por predefinição um certificado autoassinado: ative «Aceitar um certificado não verificável» a menos que o anfitrião do DumbMonit confie na sua autoridade de certificação.

## Credenciais {#credentials}

| Credencial | Campos |
|---|---|
| Nome de utilizador / palavra-passe | O utilizador dedicado com o papel Read-only: `dumbmonit@vsphere.local` num vCenter, `dumbmonit` num anfitrião. |

Endereço: um nome de anfitrião ou IP (`vcenter.lan`), `host:port`, ou um URL
completo; um `/sdk` ou `/ui` no fim é aceite. Opções: porta (443), verificação
do certificado, tempo limite do pedido (20 s) e alarmes disparados (ler).
