-- Add migration script here
ALTER TABLE urls ADD COLUMN site_name VARCHAR(255);
