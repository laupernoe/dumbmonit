# A página de dispositivos {#the-devices-page}

![O rack: uma placa de identificação por dispositivo](../assets/screenshots/devices-light.png){ loading=lazy }

## O rack {#the-rack}

Cada dispositivo é uma placa de identificação de rack 1U: um LED de status, o
nome, o selo do tipo, o endereço, quando foi visto por último, e um sparkline
de uma métrica representativa. Filhos se agrupam sob seu pai e escurecem
quando o pai está inacessível.

| Estado | Significado |
|---|---|
| Reportando | A última sondagem teve sucesso, dentro de três períodos. |
| Inacessível | O dispositivo não respondeu (tempo limite, conexão recusada), ou nenhuma sondagem por mais de três períodos (pelo menos 90 s). |
| Mal configurado | A última sondagem falhou do nosso lado: credenciais erradas, endereço ou opção inválidos, resposta inesperada. Mostrado na cor de aviso com o erro; não gera alerta. |
| Esperando | Adicionado, ainda não sondado. |
| Desativado | Pausado: não verificado, não gera alerta. |

Filtros: uma caixa de busca, um segmento de estado (Todos, Precisa de atenção,
Reportando, Desativado) e um filtro de tipo. **Adicionar dispositivo** é a ação
principal.

## Um dispositivo {#a-device}

![Uma página de dispositivo: estado, ações, alertas, o painel específico do tipo, métricas](../assets/screenshots/device-light.png){ loading=lazy }

A página do dispositivo mostra sua placa de estado, endereço, tipo, perfil
detectado, tags, última sondagem, e para serviços o status do certificado.
Ações:

| Ação | Efeito |
|---|---|
| Sondar agora | Executa uma sondagem imediatamente e informa quantas amostras e séries ela produziu. A primeira coisa a tentar quando um dispositivo não mostra dados. |
| Editar | Abre o mesmo formulário de quando foi adicionado, preenchido. Deixar a credencial vazia mantém a armazenada. |
| Silenciar 1 h / Silenciar até… | Uma janela de manutenção pontual neste dispositivo. |
| Excluir | Pergunta "Excluir definitivamente?" inline por cinco segundos, depois exclui o dispositivo. Suas séries permanecem no VictoriaMetrics até a retenção expirar. |

Abaixo: **Alertas neste dispositivo** — cada alerta ativo oferece *Reconhecer*,
*Soneca* e *Ignorar*, e uma regra ignorada neste dispositivo é listada
separadamente com **Parar de ignorar** — depois **Métricas** em um intervalo
de tempo (1 h, 6 h, 24 h, 7 dias), um gráfico por nome de métrica com uma
linha por série. Veja
[Se livrando de um alerta](alerts.md#getting-rid-of-an-alert).

Serviços (HTTP, TCP, DNS, ping, TLS) recebem instrumentos em vez disso:
**Disponibilidade** nas últimas 24 horas ou 7 dias, **Tempo de resposta**, o
número de **Verificações**, e uma barra de **Histórico** de intervalos (o mais
antigo à esquerda) mostrando quando o serviço esteve fora do ar.

## Políticas de contêiner {#container-policies}

Em uma máquina com agente e Docker, uma faixa **Docker** fica sob o
cabeçalho: contagem de contêineres, quantos estão em execução, atualizações
disponíveis, e quantos contêineres têm uma política. **Gerenciar contêineres**
abre a lista; **Políticas…** abre uma tabela com duas chaves por contêiner e
uma linha "Aplicar a todos":

- **Reiniciar se parado** — o servidor reinicia o contêiner dentro de um
  minuto após ele parar (no máximo uma vez a cada dez minutos);
- **Atualização automática em janelas de manutenção** — quando o registro tem
  uma imagem mais nova para a mesma tag, o contêiner é recriado durante uma
  janela de manutenção programada (página Alertas → Programado) e revertido
  se falhar na verificação de saúde.

Cada chave salva por conta própria. Detalhes e regras de segurança:
[Contêineres Docker](../devices/agent.md#docker-containers).

## Adicionando e editando {#adding-and-editing}

Veja [Adicione seu primeiro dispositivo](../install/first-device.md) para o
formulário e os [tipos de dispositivo](../devices/index.md) para o que cada
tipo precisa. Ao editar, o formulário abre **Mais opções** por conta própria
se o dispositivo já usa alguma.

## Excluindo {#deleting}

Excluir um dispositivo o remove da configuração, limpa seus alertas
imediatamente — nada é notificado, o histórico mantém uma linha `device
removed or disabled` — e exclui suas séries temporais do VictoriaMetrics
embutido alguns segundos depois (melhor esforço: uma falha é apenas registrada
em log, e as séries então expiram com a retenção de 12 meses). Pausar um
dispositivo limpa seus alertas da mesma forma; suas séries permanecem.
