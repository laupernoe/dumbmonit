-- Pages trouvées par le dernier parcours d'un site (`webchange`) quand
-- « Max pages » l'a arrêté : lues, et encore en attente. L'interface dit ainsi
-- qu'une partie du site n'est pas surveillée. NULL quand le parcours a tout lu
-- (ou s'est arrêté pour une autre raison).
ALTER TABLE webchange_state ADD COLUMN last_check_found INTEGER;
