# Executar uma instância de demonstração {#run-a-demo-instance}

A demonstração pública em **[demo.dumbmonit.app](https://demo.dumbmonit.app)**
é uma imagem DumbMonit normal, iniciada com `DUMBMONIT_DEMO=1`. Pode executar
a mesma coisa — para mostrar o DumbMonit a uma equipa antes de o instalar para
valer, ou para colocar uma vitrine no seu próprio site.

!!! warning "Nunca num volume de dados real"
    O modo de demonstração **recria a base de dados a cada arranque**.
    Dê-lhe um volume próprio, ou nenhum: apontado para o `/data` de uma
    instância real, substituiria os seus monitores, regras e histórico
    pelo património fictício.

## O que o modo de demonstração faz {#what-demo-mode-does}

- **Iniciar sessão com `demo` / `demo`.** A página de início de sessão diz
  isso mesmo e preenche os campos com um clique. A conta vê todos os ecrãs que
  um administrador vê.
- **Só leitura.** Todas as alterações — monitores, regras, silêncios,
  confirmações, canais, tokens, utilizadores, definições, cópias de segurança
  e restauros, análises de descoberta, registo de agentes, ferramentas de
  escrita do endpoint do assistente — são recusadas pelo servidor com `403` e
  `{"error": "This is a read-only demo…", "demo": true}`. Iniciar e terminar
  sessão são as únicas escritas permitidas. A interface mostra a mensagem no
  local onde o clique aconteceu.
- **Dois clientes fictícios.** No arranque, a base de dados é recriada e
  preenchida com um homelab em `*.home.arpa` — Proxmox VE, um NAS Synology com
  um LUN iSCSI, Proxmox Backup Server, TrueNAS, OPNsense, UniFi, Kopia, um
  servidor Redfish com uma peça avariada, um switch SNMP, sondas HTTP, TLS, DNS
  e ping, um agente Linux com Docker — e, em modo MSP, com um pequeno gabinete
  de arquitetura em Genebra: um controlador de domínio Windows com o seu
  agente, Veeam Backup & Replication, uma rede Omada, Microsoft 365 e
  Entra ID, consultados através de um agente relay instalado no escritório. Os
  monitores são respondidos por equipamentos simulados dentro do servidor,
  ligados a `127.0.0.1`, que reproduzem respostas de API gravadas: os
  coletores e painéis reais funcionam. Reiniciar dá o mesmo parque.
- **Trinta dias de histórico, e o resto do produto.** No arranque, são
  escritas métricas sintéticas no VictoriaMetrics, em blocos limitados (a
  última semana com resolução de cinco minutos): gráficos, *Últimos 7 dias*,
  disponibilidade a 30 dias, previsões, a matriz de cópias de segurança e o
  histórico de alertas ficam preenchidos, e alguns alertas estão ativos. A
  semente preenche também o que normalmente se configura à mão: equipas e
  pastas, contas e escalas de prevenção com políticas de escalonamento,
  silêncios, procedimentos, dois painéis, relatórios agendados, canais de
  notificação (nunca usados), locais no mapa-múndi, vulnerabilidades
  conhecidas, quatro páginas de estado (uma por cena noturna) com um incidente
  em curso, e uma varredura de rede com um dispositivo novo. Tudo é calculado a
  partir de uma semente fixa: nada é obtido da rede, nem sequer as fontes de
  vulnerabilidades. A pesquisa de registos fica desligada: a demo não tem
  armazenamento de registos.
- **Nada sai do servidor.** Nenhuma notificação é enviada, e nenhuma
  verificação vai a um endereço que um visitante tenha escrito.
- **Uma visita guiada** abre após a primeira sessão iniciada; a faixa reabre-a.

A limitação de taxa do início de sessão funciona como habitualmente. Coloque a
demonstração atrás do seu proxy inverso habitual com HTTPS e defina
`DUMBMONIT_COOKIE_SECURE=1` e `DUMBMONIT_TRUSTED_PROXIES` como para qualquer
instância pública.

## Iniciar {#start-it}

```bash
docker run -d --name dumbmonit-demo -p 8080:8080 \
  -e DUMBMONIT_DEMO=1 \
  ghcr.io/laupernoe/dumbmonit:latest
```

Ou com Compose, sem volume para que cada reinício comece limpo:

```yaml
services:
  dumbmonit-demo:
    image: ghcr.io/laupernoe/dumbmonit:latest
    restart: unless-stopped
    ports:
      - "8080:8080"
    environment:
      DUMBMONIT_DEMO: "1"
```

Obtenha a imagem regularmente (ou deixe uma ferramenta como o Watchtower
fazê-lo): a demonstração mostra então sempre a última versão com os mesmos
dados.
