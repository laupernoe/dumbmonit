# Mappa della topologia

La pagina **Mappa** (pulsante **Mappa** nella pagina dei monitor, *Mappa* nella
palette dei comandi, `/map`) disegna tutta l'infrastruttura in un'immagine. Nulla
si inserisce a mano: ogni collegamento viene da ciò che DumbMonit sa già.

## Da dove vengono i collegamenti

| Collegamento | Fonte | Tratto |
|---|---|---|
| Dipende da | Il **genitore** impostato su un monitor, lo stesso seguito dalla soppressione degli avvisi. | linea continua |
| Raggiunto tramite un relay | L'**agente relay** che interroga un dispositivo in un sito remoto. | trattini lunghi |
| Ospitato da un hypervisor | I guest segnalati negli ultimi 30 minuti da Proxmox VE, XCP-ng, Nutanix, vSphere o dal modulo libvirt di un agente Linux. | trattini corti |
| Cablato a (LLDP) | Un **vicino LLDP** visto da uno switch SNMP su una porta (profilo `ethernet-switch`). | puntini |

Un guest o un vicino LLDP è collegato a un monitor solo se il suo nome indica un
solo monitor (stesso nome o stesso host nell'indirizzo, dominio ignorato). Un
nome ambiguo non crea alcun collegamento invece di uno sbagliato. Un guest senza
monitor compare in un riquadro proprio accanto al suo hypervisor; un vicino LLDP
sconosciuto viene omesso. Un monitor senza collegamenti resta nella sua cartella.

## Leggere la mappa

- A monte a sinistra, ciò che ne dipende a destra; una sezione per cartella.
- Ogni riquadro mostra icona del tipo, nome e stato a parole. Un dispositivo
  **non raggiungibile** ha un bordo spesso e un badge `!`; ciò che sta dietro
  (genitore, relay o hypervisor) è segnato come **Coinvolto**, con bordo
  tratteggiato: i suoi avvisi sono soppressi.
- Un clic apre il monitor; il riquadro di un guest apre il suo hypervisor.
- Trascina per spostare, rotella o pizzico per lo zoom, frecce, `+`, `-` e `0` (adatta).
- **Mostra** limita la mappa a una cartella o al sito di un relay.
- Sul telefono la pagina si apre sulla vista **Elenco**: lo stesso albero, rientrato.

## Esportare e stampare

**Esporta** offre **Immagine SVG**, **Immagine PNG** (titolo, ambito, data e
legenda su fondo bianco) e **Stampa / PDF (A4 orizzontale)** o **(A3
orizzontale)**: il browser stampa solo quel foglio, adattato alla pagina.

## API

`GET /api/topology` restituisce nodi, collegamenti e la loro fonte (tutti i ruoli,
sola lettura, senza indirizzi). Vedi il [riferimento API](../reference/api.md).
