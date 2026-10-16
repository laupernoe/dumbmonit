# Adicionar o primeiro monitor {#add-your-first-device}

Adicionar qualquer coisa funciona sempre da mesma forma: escolha um
tipo da lista, um aviso à direita explica o que preparar no
monitor, e o formulário só mostra os campos que importam para esse
tipo.

## Os três primeiros passos {#the-three-first-steps}

Numa instância nova, a visão geral não mostra um boletim vazio: mostra
um guia curto — **add your first device**, **connect a way to be
told**, **check a message arrives**. Cada passo é um clique, e cada um
fica verde por si só. O terceiro só conta uma mensagem que realmente
saiu de um canal, por isso criar um canal e nunca o testar não o marca.

O guia é guardado na instância, não no navegador: dispensar um passo,
ou o guia inteiro, dispensa-o em todo navegador e para todo
utilizador. Desaparece de vez uma vez os três passos resolvidos, e
nunca aparece numa instância que já tinha monitores e um canal —
atualizar não entrega um tutorial a um homelab já estabelecido.

## Adicionar um dispositivo SNMP {#add-an-snmp-device}

O SNMP é o tipo mais universal: quase todo o hardware de rede o fala.
Comutadores, routers, NAS, UPS e impressoras são os suspeitos
habituais.

1. No dispositivo, ative o SNMP v2c e anote a comunidade só de leitura (`public` por predefinição em muitos dispositivos). Se o dispositivo filtrar por endereço, permita o endereço do servidor DumbMonit.
2. No DumbMonit, abra **Devices** e clique em **Add a device** (ou pressione ++ctrl+k++ e escreva «add»).
3. Escolha **SNMP device**. O seletor colapsa numa linha e o formulário mostra os campos para SNMP.
4. Introduza um **nome**, o **endereço** (IP ou nome de anfitrião) e a **comunidade**. Para SNMP v3, mude a credencial para *SNMP v3* e preencha o nome de utilizador e as frases-passe de autenticação e privacidade.
5. Clique em **Add device**.

![Adicionar um dispositivo SNMP: o formulário à esquerda, o aviso de configuração à direita](../assets/screenshots/add-device-light.png){ loading=lazy }

O perfil de coleta é detetado automaticamente a partir do `sysObjectID`
do monitor, em segundo plano, por isso o formulário nunca espera
por um monitor inacessível. Interfaces, e em servidores e NAS a
CPU, memória e armazenamento, aparecem dentro de um minuto.

!!! warning "As comunidades viajam em texto simples"
    Uma comunidade SNMP v1/v2c não é encriptada na rede. Numa rede
    partilhada, prefira o SNMP v3, que autentica e encripta.

## Analisar uma rede {#scan-a-network}

Em vez de escrever monitores um a um, analise um intervalo:

1. Em **Add a device**, abra o painel **Scan my network**.
2. Introduza uma rede em notação CIDR, por exemplo `192.168.1.0/24` (até 4096 endereços por análise), e a comunidade a tentar em cada endereço (`public` por predefinição).
3. Todo endereço que responde é listado com o seu `sysName`, descrição e o perfil que seria aplicado. Os monitores já no DumbMonit são marcados *Already added*, e tudo o que é novo está marcado para si.
4. Desmarque o que não quiser, depois clique em **Add *n* devices**. São criados um a um, cada linha mostrando o seu próprio resultado, para que uma falha nunca pare o resto.

## Adicionar um servidor Proxmox VE {#add-a-proxmox-ve-server}

O mesmo fluxo, com uma credencial em dois campos em vez de um:

1. No Proxmox, crie um utilizador reservado para monitorização, uma função só de leitura e um token de API — os quatro comandos `pveum` estão no aviso junto ao formulário, cada um com um botão de cópia, e na [página Proxmox VE](../devices/proxmox.md). Nunca reutilize a conta com que inicia sessão.
2. No DumbMonit, **Add a device** → **Proxmox VE**.
3. Introduza um **nome** e o **endereço** de qualquer nó (porta 8006 por predefinição).
4. Copie o **full-tokenid** do token (`dumbmonit@pve!monitor`) para **Token ID** e o seu **valor** (o UUID) para **Secret**. Colou a cadeia completa `user@pve!name=secret` em Token ID por engano? O formulário divide-a e diz isso.
5. O Proxmox vem com um certificado autoassinado: marque **Accept an unverifiable certificate** sob *More options* se a primeira verificação se queixar disso.
6. Clique em **Add device**.

O [Proxmox Backup Server](../devices/pbs.md) funciona da mesma forma
com `proxmox-backup-manager`; um [NAS Synology](../devices/synology.md)
usa uma conta DSM dedicada (nome de utilizador e senha); um
[servidor com o agente](../devices/agent.md) não precisa de nenhuma
credencial, só do token de registo mostrado uma vez o monitor
guardado.

## O aviso de configuração {#the-setup-notice}

Todo tipo transporta o seu próprio aviso, escrito pelo servidor e
mostrado junto ao formulário: passos numerados a fazer no dispositivo
(criar um utilizador só de leitura, criar um token de API, permitir o
endereço…), com o caminho exato de menu no produto e um botão de cópia
em cada comando ou valor, depois uma armadilha comum e uma ligação à
documentação do fornecedor quando existe. As páginas por tipo nesta
documentação transportam os mesmos passos palavra por palavra — um
teste no servidor mantém-nos idênticos: [SNMP](../devices/snmp.md),
[Proxmox VE](../devices/proxmox.md),
[Proxmox Backup Server](../devices/pbs.md),
[Synology DSM](../devices/synology.md), [agente](../devices/agent.md),
[serviços](../devices/services.md).

O princípio por detrás de todo aviso: uma conta dedicada só de
leitura, nunca aquela com que inicia sessão. Um segredo de
monitorização exposto não deve poder alterar nada.

## Campos comuns {#common-fields}

Qualquer que seja o tipo, o formulário tem:

| Campo | Significado |
|---|---|
| Name | Como o monitor aparece em listas e alertas. |
| Address | IP, nome de anfitrião, `anfitrião:porta` ou URL dependendo do tipo; o marcador de posição mostra a forma esperada. |
| Credential | As famílias que o tipo aceita, cada uma com os seus próprios campos: comunidade, utilizador e frases-passe SNMP v3, token Proxmox (Token ID + Secret), nome de utilizador e senha. Os segredos têm um olho de mostrar/esconder e os valores colados são limpos de espaços, aspas e quebras de linha indesejados. Guardados encriptados, nunca devolvidos pela API: ao editar, campos vazios mantêm o que está guardado. |
| Check interval | Com que frequência o DumbMonit lê o monitor. Predefinição 60 s, mínimo 10 s. |
| Parent device | Se o pai cair, os alertas deste monitor são suprimidos em vez de enviados. Ver [supressão por dependência](../alerting/index.md#dependency-suppression). |
| Enabled | Um monitor pausado não é verificado e não gera nenhum alerta. |
| Tags | Etiquetas livres de chave/valor, copiadas em cada série como `tag_<key>`. As opções específicas do tipo também vivem aqui, sob **More options**. |

## O que acontece a seguir {#what-happens-next}

- O monitor aparece na página **Devices** como uma placa frontal
  de rack: LED de estado, nome, tipo, endereço, visto por último, e um
  sparkline.
- As medições são escritas na VictoriaMetrics a cada 5 segundos
  (`DUMBMONIT_FLUSH_INTERVAL_SECS`), por isso o primeiro gráfico
  aparece depois da primeira verificação mais alguns segundos.
  **Probe now** na página do monitor força uma verificação e
  reporta quantas amostras e séries produziu: é a principal ferramenta
  de diagnóstico.
- As [regras de alerta incorporadas](../alerting/rules.md) aplicam-se
  de imediato: monitor inacessível, CPU saturada, disco quase
  cheio, e assim por diante. Nada para configurar.
- Para ser notificado, adicione um
  [canal de notificação](../notifications.md) em **Settings →
  Notifications & reports**. As regras incorporadas notificam todo canal
  ativado.

!!! tip "Sem dados depois de um minuto?"
    Abra a página do monitor: um erro de configuração (comunidade
    errada, certificado recusado, capacidade em falta) é mostrado em
    vermelho sob o endereço, com o motivo. Ver as
    [Perguntas frequentes](../faq.md#no-data-after-adding-a-device).
