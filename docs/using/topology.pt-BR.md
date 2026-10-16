# Mapa de topologia

A página **Mapa** (botão **Mapa** na página de monitores, *Mapa* na paleta de
comandos, `/map`) desenha toda a sua infraestrutura em uma imagem. Nada é
digitado à mão: cada vínculo vem do que o DumbMonit já sabe.

## De onde vêm os vínculos

| Vínculo | Origem | Traço |
|---|---|---|
| Depende de | O **pai** definido em um monitor, o mesmo seguido pela supressão de alertas. | linha contínua |
| Acessado por um relay | O **agente relay** que consulta um monitor em um site remoto. | traços longos |
| Hospedado por um hipervisor | Os convidados informados nos últimos 30 minutos pelo Proxmox VE, XCP-ng, Nutanix, vSphere ou pelo módulo libvirt de um agente Linux. | traços curtos |
| Cabeado a (LLDP) | Um **vizinho LLDP** visto por um switch SNMP em uma porta (perfil `ethernet-switch`). | pontos |

Um convidado ou vizinho LLDP só é vinculado a um monitor se o nome designar um
único monitor (mesmo nome ou mesmo host no endereço, domínio ignorado). Um nome
ambíguo não cria vínculo algum em vez de um errado. Um convidado sem monitor
aparece em sua própria caixa ao lado do hipervisor; um vizinho LLDP
desconhecido é omitido. Um monitor sem vínculos fica em sua pasta.

## Lendo o mapa

- O que está acima à esquerda, o que depende à direita; uma seção por pasta.
- Cada caixa mostra o ícone do tipo, o nome e o estado por extenso. Um
  monitor **fora do ar** tem contorno grosso e um selo `!`; o que está atrás
  dele (pai, relay ou hipervisor) é marcado como **Afetado**, com contorno
  tracejado: seus alertas são suprimidos.
- Um clique abre o monitor; a caixa de um convidado abre o hipervisor.
- Arraste para mover, role ou faça pinça para o zoom, setas, `+`, `-` e `0` (ajustar).
- **Mostrar** limita o mapa a uma pasta ou ao site de um relay.
- No celular a página abre na visualização **Lista**: a mesma árvore, recuada.

## Exportar e imprimir

**Exportar** oferece **Imagem SVG**, **Imagem PNG** (título, escopo, data e
legenda em fundo branco) e **Imprimir / PDF (A4 paisagem)** ou **(A3
paisagem)**: o navegador imprime só essa folha, ajustada à página.

## API

`GET /api/topology` retorna nós, vínculos e a origem de cada um (qualquer papel,
somente leitura, sem endereços). Veja a [referência da API](../reference/api.md).
