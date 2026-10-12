# Dispositivo demo

Un dispositivo fittizio che produce misurazioni, per esplorare lo strumento
senza hardware. Nessun hardware richiesto.

## Configurazione

1. Dagli un nome qualsiasi e confermalo.
2. Misurazioni finte appariranno immediatamente, a sufficienza per esplorare
   l'interfaccia.

L'indirizzo è libero (il segnaposto indica `demo`), non ci sono credenziali né
opzioni. Il dispositivo demo produce una serie `cpu_usage_percent`, quindi le
regole "High CPU" e "Unusual CPU" si applicano ad esso, ed è possibile usarlo
come genitore o figlio in una [catena di dipendenza](../alerting/index.md#dependency-suppression)
per vedere la soppressione in azione.

Per misurazioni reali senza hardware dedicato, punta un dispositivo SNMP verso
qualsiasi macchina che esegue `snmpd`: vedi [Testare senza hardware](../install/docker.md#testing-without-hardware).
