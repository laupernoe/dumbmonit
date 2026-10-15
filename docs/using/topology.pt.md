# Mapa de topologia

A página **Mapa** (botão **Mapa** na página de monitores, *Mapa* na paleta de
comandos, `/map`) desenha toda a sua infraestrutura numa imagem. Nada é
introduzido à mão: cada ligação vem do que o DumbMonit já sabe.

## De onde vêm as ligações

| Ligação | Origem | Traço |
|---|---|---|
| Depende de | O **pai** definido num monitor, o mesmo que a supressão de alertas segue. | linha contínua |
| Alcançado através de um relé | O **agente relé** que consulta um equipamento num local remoto. | traços longos |
| Alojado num hipervisor | Os convidados reportados nos últimos 30 minutos pelo Proxmox VE, XCP-ng, Nutanix, vSphere ou pelo módulo libvirt de um agente Linux. | traços curtos |
| Ligado por cabo a (LLDP) | Um **vizinho LLDP** visto por um switch SNMP numa porta (perfil `ethernet-switch`). | pontos |

Um convidado ou vizinho LLDP só é ligado a um monitor se o seu nome designar um
único monitor (mesmo nome ou mesmo nome de anfitrião no endereço, domínio
ignorado). Um nome ambíguo não cria nenhuma ligação em vez de uma errada. Um
convidado sem monitor aparece na sua própria caixa junto ao hipervisor; um
vizinho LLDP desconhecido é omitido. Um monitor sem ligações fica na sua pasta.

## Ler o mapa

- A montante à esquerda, o que depende à direita; uma secção por pasta.
- Cada caixa mostra o ícone do tipo, o nome e o estado por extenso. Um
  equipamento **em baixo** tem contorno grosso e um distintivo `!`; o que está
  atrás dele (pai, relé ou hipervisor) fica marcado como **Afetado**, com
  contorno tracejado: os seus alertas são suprimidos.
- Um clique abre o monitor; a caixa de um convidado abre o seu hipervisor.
- Arraste para mover, roda ou pinça para ampliar, setas, `+`, `-` e `0` (ajustar).
- **Mostrar** limita o mapa a uma pasta ou ao local de um relé.
- No telemóvel a página abre na vista **Lista**: a mesma árvore, indentada.

## Exportar e imprimir

**Exportar** oferece **Imagem SVG**, **Imagem PNG** (título, âmbito, data e
legenda sobre fundo branco) e **Imprimir / PDF (A4 horizontal)** ou **(A3
horizontal)**: o navegador imprime só essa folha, ajustada à página.

## API

`GET /api/topology` devolve nós, ligações e a origem de cada uma (qualquer
função, só leitura, sem endereços). Veja a [referência da API](../reference/api.md).
