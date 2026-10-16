# FortiGate {#fortigate}

Um Fortinet FortiGate através da API REST do FortiOS: túneis IPsec
site-to-site fora do ar, interfaces ativadas que perderam seu link, um
cluster HA fora de sincronia ou com falta de um membro, memória se
aproximando do modo de conservação, licenças FortiGuard e FortiCare
expirando, e versões de patch oferecidas para o branch FortiOS em
execução.

O DumbMonit usa um usuário de API REST cujo perfil de acesso só lê, e que
só aceita chamadas do endereço do servidor DumbMonit (host confiável). Seu
token é enviado como `Authorization: Bearer`, nunca na URL: o FortiOS
7.4.5 e 7.6.1 o recusam lá de qualquer forma.

A cada minuto o DumbMonit lê três páginas obrigatórias —
`/api/v2/monitor/system/status`, `/api/v2/monitor/system/resource/usage`
e `/api/v2/monitor/system/interface/select` — e seis opcionais que um
perfil mais restrito, ou um FortiGate sem cluster ou túneis, pode recusar
ou deixar vazias sem falhar o resto: o estado administrativo de
interfaces (`/api/v2/cmdb/system/interface`), estatísticas e checksums de
HA, túneis IPsec, status de licença e firmware disponível.

!!! warning "Validado apenas contra a documentação do fornecedor — ainda não testado em um sistema real"

    Esta integração foi construída a partir da documentação do FortiOS da
    Fortinet e das formas de resposta registradas pelo exportador
    Prometheus da comunidade para o FortiGate, não contra um FortiGate em
    execução. Toda chamada é uma leitura. As páginas de licença e firmware
    são as menos documentadas: avise-nos o que não funcionar.

## O que ele observa {#what-it-watches}

Todas as métricas têm o prefixo `dumbmonit_fortigate_`.

| Métrica | Rótulos | Significado |
|---|---|---|
| `version_info` | `version`, `build`, `model`, `hostname` | Valor 1. |
| `cpu_usage_percent`, `memory_used_percent`, `disk_used_percent` | | Uso atual. |
| `sessions`, `session_setup_rate` | | Sessões ativas, novas sessões por segundo. |
| `log_disk_ok` | | 0 quando o disco de log precisa ser formatado. |
| `interface_link_up` | `interface`, `alias` | Link físico. |
| `interface_enabled` | `interface`, `alias` | Estado administrativo; toda interface conta como ativada quando o perfil não consegue ler a configuração. |
| `interface_down` | `interface`, `alias` | 1 quando a interface está ativada, tem um endereço e não tem link. Portas livres sem endereço são deixadas de lado. |
| `interface_speed_mbps` | `interface`, `alias` | Enquanto o link está ativo. |
| `interface_bytes_in_total`, `interface_bytes_out_total`, `interface_errors_in_total`, `interface_errors_out_total` | `interface`, `alias` | Contadores. |
| `interfaces_down` | | Contagem. |
| `ha_members` | | Membros no cluster; 0 para um FortiGate autônomo. |
| `ha_in_sync` | | Com dois membros ou mais: 1 quando todo membro tem o mesmo checksum de configuração. |
| `ha_member_cpu_usage_percent`, `ha_member_memory_used_percent` | `member` | Por membro. |
| `ipsec_tunnel_up` | `tunnel` | 1 quando ao menos uma fase 2 do túnel está ativa. Túneis dial-up (acesso remoto) são deixados de fora. |
| `ipsec_phase2_up` | `tunnel`, `phase2` | Por seletor de fase 2. |
| `ipsec_tunnel_bytes_in_total`, `ipsec_tunnel_bytes_out_total` | `tunnel` | Contadores. |
| `ipsec_tunnel_info` | `tunnel`, `remote_gateway` | Valor 1. |
| `ipsec_tunnels`, `ipsec_tunnels_down` | | Contagens. |
| `license_status_info` | `license`, `status` | Valor 1, para toda licença que o FortiGate possui (`antivirus`, `ips`, `web_filtering`, `forticare_hardware`…); entradas `no_license` são deixadas de fora. |
| `license_expired` | `license` | 1 quando o status é `expired`. |
| `license_expiry_seconds` | `license` | Até a expiração; negativo uma vez expirado. |
| `firmware_update_available` | | 1 quando o FortiGuard oferece uma versão de patch mais nova do branch em execução (7.4.4 → 7.4.5). Um novo branch (7.6) é um projeto, não um alerta. |
| `firmware_latest_info` | `version` | Valor 1: essa versão de patch. |

As [regras integradas](../alerting/rules.md#fortigate) que se aplicam:

- **FortiGate IPsec tunnel down**: nenhuma fase 2 ativa por cinco minutos
  (Warning).
- **FortiGate interface without link**: por cinco minutos (Advisory).
- **FortiGate HA out of sync**: por quinze minutos (Advisory).
- **FortiGate HA member lost**: menos membros que em qualquer momento nas
  últimas 24 horas (Warning). A contagem esperada não é publicada em
  lugar nenhum, então um membro removido permanentemente para de alertar
  no dia seguinte.
- **FortiGate memory near conserve mode**: acima de 85% por dez minutos
  (Advisory). A 88% por padrão o FortiOS entra em modo de conservação e
  para de inspecionar novas sessões.
- **FortiGate CPU high**: acima de 90% por quinze minutos (Advisory).
- **FortiGate licence expiring**: menos de 30 dias restantes, ou expirada
  (Advisory, lembrada diariamente).
- **FortiGate firmware update**: uma versão de patch é oferecida (Info,
  lembrada semanalmente).

## A página do monitor {#the-device-page}

O painel Firewall diz o que está errado em uma frase, mostra CPU, memória,
sessões e o estado HA, depois lista os túneis IPsec (os fora do ar
primeiro), as interfaces sem link, as licenças expirando dentro de 60 dias
ou expiradas, e uma versão de patch disponível.

## Criar um usuário de API REST somente leitura para o DumbMonit {#create-a-read-only-rest-api-user-for-dumbmonit}

1. Na CLI do FortiGate, crie um perfil de acesso que só pode ler o estado
   de sistema, rede e VPN, e nada mais.

    ```
    config system accprofile
    edit "dumbmonit-ro"
    set sysgrp read
    set netgrp read
    set vpngrp read
    set fwgrp none
    set loggrp none
    set utmgrp none
    set wanoptgrp none
    set wifi none
    next
    end
    ```

2. Crie o usuário de API REST dumbmonit com esse perfil, confiando apenas
   no endereço do servidor DumbMonit (substitua 192.0.2.10), depois gere
   seu token: ele é impresso uma vez.

    ```
    config system api-user
    edit "dumbmonit"
    set accprofile "dumbmonit-ro"
    set vdom "root"
    config trusthost
    edit 1
    set ipv4-trusthost 192.0.2.10 255.255.255.255
    next
    end
    next
    end
    execute api-user generate-key dumbmonit
    ```

3. No DumbMonit, informe o endereço do FortiGate, por exemplo "fw.lan"
   (adicione a porta se a interface de administração não escutar na
   443), e cole o token como token de API. O FortiGate vem com um
   certificado autoassinado: marque Aceitar um certificado não
   verificável a menos que você tenha instalado o seu próprio.

!!! warning
    Uma interface conta como fora do ar apenas quando está ativada, tem
    um endereço e não tem link: portas livres são deixadas de lado.
    Atualizações de firmware são as que o FortiGuard oferece ao próprio
    FortiGate: sem acesso ao FortiGuard, nenhuma é reportada.

## Credenciais {#credentials}

| Credencial | Campos |
|---|---|
| Token de API | O token impresso por `execute api-user generate-key dumbmonit`, enviado como `Authorization: Bearer`. |

Endereço: o nome de host ou IP do FortiGate, `host:porta`, ou uma URL.
Opções:

- **Porta** (443): a porta HTTPS da interface de administração (`set
  admin-sport` sob `config system global`).
- **VDOM**: o domínio virtual a ler; vazio para aquele a que o usuário de
  API REST pertence.
- **Aceitar um certificado não verificável**: o próprio certificado do
  FortiGate é autoassinado.
- **Tempo limite por requisição** (15 s).

## Solução de problemas {#troubleshooting}

"FortiGate refused the API token (401)": o token está errado ou foi
regenerado. Gere um novo com `execute api-user generate-key dumbmonit`.

"FortiGate refused … (403)": o endereço do DumbMonit não está entre os
hosts confiáveis do usuário, ou seu perfil não pode ler aquela página.
Uma página opcional recusada assim é simplesmente deixada de fora.

Nenhum número de IPsec, licença ou firmware: o perfil não pode lê-los, ou
o FortiGate não tem túnel; o resto ainda é lido.
