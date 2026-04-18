ALTER TABLE classifier_blocks
DROP CONSTRAINT classifier_blocks_order_key;

ALTER TABLE classifier_blocks
ADD CONSTRAINT classifier_blocks_order_key
UNIQUE ("order");
