-- Pages de statut : Dubaï remplace le Cervin parmi les quatre scènes de
-- bannière (paris, tokyo, newyork, dubai). `matterhorn` devient `dubai` ; le
-- REPLACE est imbriqué deux fois car deux identifiants voisins partagent leur
-- virgule. Les doublons éventuels (`matterhorn,dubai`) sont écartés à la
-- lecture (`split_scenes`).
UPDATE status_pages
SET scenes = TRIM(
    REPLACE(REPLACE(',' || scenes || ',', ',matterhorn,', ',dubai,'), ',matterhorn,', ',dubai,'),
    ','
)
WHERE ',' || scenes || ',' LIKE '%,matterhorn,%';
