# Modo parede {#wall-mode-and-music}

`/wall` mostra o quadro próprio da consola, em ecrã inteiro, para um monitor que
fica ligado numa sala. Atualiza-se sozinho e mantém o ecrã acordado.

De cima para baixo:

- o **veredicto** em poucas palavras («All good», «3 problems») com a data e o
  relógio, e quando foi atualizado pela última vez;
- os **cinco números** do resumo: monitores ativos, disponibilidade a 30 dias,
  tempo de resposta p95, alertas abertos e tempo médio de resolução;
- o **rio de saúde**, a grande imagem do quadro: uma faixa por monitor, os piores
  no topo;
- os **alertas abertos**, os mais graves primeiro;
- **quem está de prevenção** agora, com a escala e o fim do turno.

Sem música e sem widgets: fundo preto, a tipografia da consola,
estados escritos junto de uma forma e nunca só por cor.

## Abrir e sair

Abra-o com o ícone **Wall mode** da barra superior, a partir da paleta de
comandos («Wall mode») ou escrevendo o URL. ++esc++ ou **Exit** volta à consola.
Uma conta limitada a algumas equipas ou clientes não tem o modo parede, porque
mostra toda a frota.

## Fundo {#backdrop}

A parede partilha o aspeto escuro da consola. O botão **Fundo** (em cima à
direita, ao lado de Exit) pode desenhar uma das cidades à noite das
[páginas de estado](status-pages.md) por trás do veredicto e do relógio: Paris,
Nova Iorque, Tóquio ou Dubai, ou *Auto* para a mais próxima do fuso horário do
navegador. Está desligado por predefinição e é lembrado por navegador;
`?city=paris` (ou `newyork`, `tokyo`, `dubai`, `none`) numa ligação escolhe-o só
para esse ecrã. O céu segue o painel: estrelas quando está tudo bem, nuvens com
um aviso, trovoada com um alerta. Num telemóvel, ou com o separador escondido, a
cidade fica parada. Um `zermatt` guardado ou numa ligação vai agora dar ao
Dubai.
