# Executar uma instância de demonstração {#run-a-demo-instance}

A demo pública em **[demo.dumbmonit.app](https://demo.dumbmonit.app)** é uma
imagem comum do DumbMonit iniciada com `DUMBMONIT_DEMO=1`. Você pode executar a
mesma coisa — para mostrar o DumbMonit a uma equipe antes de instalá-lo de
verdade, ou para colocar uma vitrine no seu próprio site.

!!! warning "Nunca em um volume de dados real"
    O modo demo **recria o banco de dados a cada inicialização**. Dê a ele um
    volume próprio, ou nenhum: apontado para o `/data` de uma instância real,
    ele substituiria seus monitores, regras e histórico pelo acervo fictício.

## O que o modo demo faz {#what-demo-mode-does}

- **Entre com `demo` / `demo`.** A página de login avisa isso e preenche os
  campos com um clique. A conta vê todas as telas que um administrador vê.
- **Somente leitura.** Toda alteração — monitores, regras, silêncios,
  reconhecimentos, canais, tokens, usuários, configurações, backups e
  restaurações, varreduras de descoberta, cadastro de agentes, ferramentas de
  escrita do endpoint do assistente — é recusada pelo servidor com `403` e
  `{"error": "This is a read-only demo…", "demo": true}`. Entrar e sair são as
  únicas escritas permitidas. A interface mostra a mensagem onde o clique
  aconteceu.
- **Dois clientes fictícios.** Na inicialização, o banco de dados é recriado e
  preenchido com um homelab em `*.home.arpa` — Proxmox VE, um NAS Synology com
  um LUN iSCSI, Proxmox Backup Server, TrueNAS, OPNsense, UniFi, Kopia, um
  servidor Redfish com uma peça com defeito, um switch SNMP, verificações HTTP,
  TLS, DNS e ping, um agente Linux rodando Docker — e, no modo MSP, com um
  pequeno escritório de arquitetura em Genebra: um controlador de domínio
  Windows com seu agente, Veeam Backup & Replication, uma rede Omada,
  Microsoft 365 e Entra ID, consultados por um agente relay instalado no
  escritório. Os monitores são respondidos por equipamentos simulados dentro
  do servidor, ligados a `127.0.0.1`, que reproduzem respostas de API
  gravadas: os coletores e painéis reais funcionam. Reiniciar dá o mesmo
  parque.
- **Trinta dias de histórico, e o resto do produto.** Na inicialização,
  métricas sintéticas são gravadas no VictoriaMetrics, em blocos limitados (a
  última semana com resolução de cinco minutos): gráficos, *Últimos 7 dias*,
  disponibilidade em 30 dias, previsões, a matriz de backups e o histórico de
  alertas ficam preenchidos, e alguns alertas estão ativos. A semente também
  preenche o que normalmente se configura à mão: equipes e pastas, contas e
  escalas de plantão com políticas de escalonamento, silêncios, runbooks, dois
  painéis, relatórios agendados, canais de notificação (nunca usados), locais
  no mapa-múndi, vulnerabilidades conhecidas, quatro páginas de status (uma
  por cena noturna) com um incidente em andamento, e uma varredura de rede com
  um dispositivo novo. Tudo é calculado a partir de uma semente fixa: nada é
  baixado da rede, nem mesmo as fontes de vulnerabilidades. A busca de logs
  fica desligada: a demo não tem armazenamento de logs.
- **Nada sai do servidor.** Nenhuma notificação é enviada, e nenhuma sondagem
  vai a um endereço que um visitante tenha digitado.
- **Um tour guiado** abre após o primeiro login; o banner o reabre.

A limitação de taxa do login funciona normalmente. Coloque a demo detrás do
seu proxy reverso habitual com HTTPS e defina `DUMBMONIT_COOKIE_SECURE=1` e
`DUMBMONIT_TRUSTED_PROXIES` como em qualquer instância pública.

## Inicie-a {#start-it}

```bash
docker run -d --name dumbmonit-demo -p 8080:8080 \
  -e DUMBMONIT_DEMO=1 \
  ghcr.io/laupernoe/dumbmonit:latest
```

Ou com Compose, sem volume, para que cada reinício comece do zero:

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

Baixe a imagem regularmente (ou deixe uma ferramenta como o Watchtower fazer
isso): a demo então sempre mostra o lançamento mais recente com os mesmos
dados.
