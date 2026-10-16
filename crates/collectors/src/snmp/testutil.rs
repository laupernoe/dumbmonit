//! Outillage partagé par les tests du collecteur SNMP.

/// Exécute une future sur un fil dédié, doté d'une pile large.
///
/// `snmp2::AsyncSession` embarque ses tampons d'émission et de réception en ligne,
/// soit cent trente kilo-octets ; les copies que la compilation de débogage insère à
/// sa construction dépassent la pile de deux mégaoctets d'un fil de test ordinaire.
/// Le correctif de fond est la fonctionnalité `heap_buffers` du crate `snmp2`, qui
/// relève du manifeste et non de ce module.
pub fn block_on_large_stack<F, T>(future: F) -> T
where
    F: std::future::Future<Output = T> + Send + 'static,
    T: Send + 'static,
{
    std::thread::Builder::new()
        .stack_size(8 * 1024 * 1024)
        .spawn(move || {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("construction du runtime de test")
                .block_on(future)
        })
        .expect("création du fil de test")
        .join()
        .expect("le fil de test a paniqué")
}

/// Agent SNMPv1 factice, sur la boucle locale, qui ne répond qu'aux GET.
///
/// Il reproduit le comportement des agents v1 réels : un seul OID inconnu fait
/// refuser tout le lot par `noSuchName`, la position fautive (à partir de 1) dans
/// `error_index`. Les `drop_first` premiers datagrammes sont ignorés, comme s'ils
/// s'étaient perdus en route.
pub struct FakeAgent {
    pub address: String,
    /// Nombre de datagrammes reçus, perdus compris.
    pub requests: std::sync::Arc<std::sync::atomic::AtomicUsize>,
}

impl FakeAgent {
    pub async fn spawn(known: Vec<(&str, i64)>, drop_first: usize) -> Self {
        Self::spawn_with(known, drop_first, 0, std::time::Duration::ZERO).await
    }

    /// Comme [`FakeAgent::spawn`], mais les `late_first` premières réponses partent
    /// avec `delay` de retard au lieu d'être perdues : l'agent lent dont la réponse
    /// arrive après l'abandon de la requête.
    pub async fn spawn_late(
        known: Vec<(&str, i64)>,
        late_first: usize,
        delay: std::time::Duration,
    ) -> Self {
        Self::spawn_with(known, 0, late_first, delay).await
    }

    async fn spawn_with(
        known: Vec<(&str, i64)>,
        drop_first: usize,
        late_first: usize,
        delay: std::time::Duration,
    ) -> Self {
        use std::sync::atomic::Ordering;

        let known: std::collections::BTreeMap<super::oid::ObjectId, i64> =
            known.into_iter().map(|(oid, value)| (oid.parse().unwrap(), value)).collect();
        let socket = std::sync::Arc::new(tokio::net::UdpSocket::bind("127.0.0.1:0").await.unwrap());
        let address = socket.local_addr().unwrap().to_string();
        let requests = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let counter = requests.clone();

        tokio::spawn(async move {
            let mut buffer = vec![0u8; 65_535];
            loop {
                let Ok((len, peer)) = socket.recv_from(&mut buffer).await else { return };
                let seen = counter.fetch_add(1, Ordering::SeqCst);
                if seen < drop_first {
                    continue;
                }
                let Ok(request) = snmp2::Pdu::from_bytes(&buffer[..len]) else { continue };
                let oids: Vec<super::oid::ObjectId> = request
                    .varbinds
                    .clone()
                    .filter_map(|(oid, _)| super::oid::ObjectId::from_snmp(&oid))
                    .collect();
                let missing = oids.iter().position(|oid| !known.contains_key(oid));
                let (status, index) = match missing {
                    Some(position) => (2, position as i64 + 1),
                    None => (0, 0),
                };
                let varbinds: Vec<u8> = oids
                    .iter()
                    .flat_map(|oid| {
                        let value = match (missing, known.get(oid)) {
                            (None, Some(value)) => tlv(0x02, &integer(*value)),
                            _ => vec![0x05, 0x00],
                        };
                        tlv(0x30, &[tlv(0x06, &oid_body(oid.arcs())), value].concat())
                    })
                    .collect();
                let pdu = [
                    tlv(0x02, &integer(request.req_id as i64)),
                    tlv(0x02, &integer(status)),
                    tlv(0x02, &integer(index)),
                    tlv(0x30, &varbinds),
                ]
                .concat();
                let message = tlv(
                    0x30,
                    &[tlv(0x02, &integer(0)), tlv(0x04, request.community), tlv(0xa2, &pdu)]
                        .concat(),
                );
                if seen < drop_first + late_first {
                    let socket = socket.clone();
                    tokio::spawn(async move {
                        tokio::time::sleep(delay).await;
                        let _ = socket.send_to(&message, peer).await;
                    });
                } else {
                    let _ = socket.send_to(&message, peer).await;
                }
            }
        });
        Self { address, requests }
    }
}

/// Encodage BER minimal : étiquette, longueur, contenu.
fn tlv(tag: u8, content: &[u8]) -> Vec<u8> {
    let mut out = vec![tag];
    let len = content.len();
    if len < 0x80 {
        out.push(len as u8);
    } else if len <= 0xff {
        out.extend([0x81, len as u8]);
    } else {
        out.extend([0x82, (len >> 8) as u8, len as u8]);
    }
    out.extend_from_slice(content);
    out
}

fn integer(value: i64) -> Vec<u8> {
    let bytes = value.to_be_bytes();
    let mut start = 0;
    while start < 7
        && ((bytes[start] == 0 && bytes[start + 1] & 0x80 == 0)
            || (bytes[start] == 0xff && bytes[start + 1] & 0x80 != 0))
    {
        start += 1;
    }
    bytes[start..].to_vec()
}

fn oid_body(arcs: &[u64]) -> Vec<u8> {
    let mut out = vec![(arcs[0] * 40 + arcs[1]) as u8];
    for &arc in &arcs[2..] {
        let mut chunk = vec![(arc & 0x7f) as u8];
        let mut rest = arc >> 7;
        while rest > 0 {
            chunk.push((rest & 0x7f) as u8 | 0x80);
            rest >>= 7;
        }
        chunk.reverse();
        out.extend(chunk);
    }
    out
}
