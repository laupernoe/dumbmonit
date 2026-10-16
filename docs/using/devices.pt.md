# A página de monitores {#the-devices-page}

![O bastidor: uma placa frontal por monitor](../assets/screenshots/devices-light.png){ loading=lazy }

## O bastidor {#the-rack}

Cada monitor é uma placa frontal de rack 1U: um LED de estado, o nome, o
selo do tipo, o endereço, quando foi visto por último, e um sparkline de uma
métrica representativa. Os filhos empilham-se sob o seu pai e esbatem-se
quando o pai está inacessível.

| Estado | Significado |
|---|---|
| Reporting | A última verificação teve sucesso, dentro de três períodos. |
| Unreachable | O monitor não respondeu (tempo esgotado, ligação recusada), ou nenhuma verificação há mais de três períodos (pelo menos 90 s). |
| Misconfigured | A última verificação falhou do nosso lado: credenciais erradas, endereço ou opção inválidos, resposta inesperada. Mostrado na cor de aviso com o erro; não gera nenhum alerta. |
| Waiting | Adicionado, ainda não verificado. |
| Disabled | Em pausa: não verificado, não gera nenhum alerta. |

Filtros: uma caixa de pesquisa, um segmento de estado (All, Needs attention,
Reporting, Disabled) e um filtro por tipo. **Add a device** é a ação principal.

O botão **Select** (operadores e administradores) ativa uma caixa de seleção por
monitor e uma barra flutuante em baixo com as ações em lote; **Done** ou
Escape sai do modo.

## Um monitor {#a-device}

![Página de um monitor: estado, ações, alertas, o painel específico do tipo, métricas](../assets/screenshots/device-light.png){ loading=lazy }

A página do monitor mostra a sua placa de estado, endereço, tipo, perfil
detetado, etiquetas, última verificação, e para serviços o estado do
certificado. Ações:

| Ação | Efeito |
|---|---|
| Probe now | Executa uma verificação de imediato e informa quantas amostras e séries produziu. A primeira coisa a tentar quando um monitor não mostra dados. |
| Edit | Abre o mesmo formulário usado ao adicionar, pré-preenchido. Deixar a credencial vazia mantém a guardada. |
| Silence 1 h / Silence until… | Uma janela de manutenção pontual neste monitor. |
| Delete | Pergunta «Delete for good?» em linha durante cinco segundos, depois elimina o monitor. As suas séries permanecem na VictoriaMetrics até a retenção expirar. |

Abaixo: **Alerts on this device** — cada alerta ativo oferece *Ack*, *Snooze*
e *Ignore*, e uma regra ignorada neste monitor é listada em separado com
**Stop ignoring** — depois **Metrics** ao longo de um intervalo de tempo (1 h,
6 h, 24 h, 7 dias), um gráfico por nome de métrica com uma linha por série.
Ver [Livrar-se de um alerta](alerts.md#getting-rid-of-an-alert).

Os serviços (HTTP, TCP, DNS, ping, TLS) têm instrumentos diferentes:
**Availability** nas últimas 24 horas ou 7 dias, **Response time**, o número
de **Checks**, e uma barra de **History** com ranhuras (as mais antigas à
esquerda) a mostrar quando o serviço esteve em baixo.

## Políticas de contentores {#container-policies}

Numa máquina com agente e Docker, uma faixa **Docker** fica sob o cabeçalho:
número de contentores, quantos estão em execução, atualizações disponíveis, e
quantos contentores têm uma política. **Manage containers** abre a lista;
**Policies…** abre uma tabela com dois interruptores por contentor e uma linha
«Apply to all»:

- **Restart if down** — o servidor reinicia o contentor dentro de um minuto
  após este parar (no máximo uma vez a cada dez minutos);
- **Auto-update in maintenance windows** — quando o registo tem uma imagem
  mais recente para a mesma etiqueta, o contentor é recriado durante uma
  janela de manutenção agendada (página Alerts → Maintenance) e revertido se
  falhar a sua verificação de saúde.

Cada interruptor guarda por si só. Detalhes e regras de segurança:
[Contentores Docker](../devices/agent.md#docker-containers).

## Adicionar e editar {#adding-and-editing}

Ver [Adicionar o primeiro monitor](../install/first-device.md) para o
formulário e os [tipos de monitor](../devices/index.md) para o que cada
tipo necessita. Ao editar, o formulário abre **More options** por si só se o
monitor já usar uma.

## Eliminar {#deleting}

Eliminar um monitor remove-o da configuração, limpa os seus alertas na
hora — nada é notificado, o histórico mantém uma linha `device removed or
disabled` — e elimina as suas séries temporais da VictoriaMetrics incorporada
alguns segundos depois (melhor esforço: uma falha é apenas registada, e as
séries envelhecem depois com a retenção de 12 meses). Pausar um monitor
limpa os seus alertas da mesma forma; as suas séries permanecem.
