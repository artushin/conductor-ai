-- Migration 038: add optional workflow column to tickets.
-- Allows ticket creators to specify which workflow should execute the ticket,
-- bypassing routing heuristics.
ALTER TABLE tickets ADD COLUMN workflow TEXT;
