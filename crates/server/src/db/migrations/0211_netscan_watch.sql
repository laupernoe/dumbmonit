-- Surveillance du sous-réseau (`netinventory/`) : appareils ignorés et
-- changement de MAC pour une IP.
--
-- `ignored` : l'administrateur ne veut plus entendre parler de cet appareil
-- (invité, imprimante de passage). Il reste « connu » pour tout le reste du
-- code (`known = 1`) ; ce drapeau ne fait que le distinguer d'un appareil
-- validé, et couper les alertes de ports nouveaux.
--
-- `mac_change_from` : MAC qu'avait jusqu'ici l'IP de cet appareil nouveau, quand
-- une autre MAC l'occupait déjà dans l'inventaire (usurpation ARP ou
-- remplacement de matériel). Vide dès que l'appareil est marqué connu ou ignoré.
ALTER TABLE netscan_devices ADD COLUMN ignored INTEGER NOT NULL DEFAULT 0;
ALTER TABLE netscan_devices ADD COLUMN mac_change_from TEXT;

CREATE INDEX idx_netscan_devices_mac_change ON netscan_devices(mac_change_from)
    WHERE mac_change_from IS NOT NULL;
