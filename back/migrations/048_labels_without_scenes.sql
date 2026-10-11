-- campaign/pin-scenes-on-maps — a map's label is only words.
--
-- A label used to name the scene its place opens (`labels[].scene`).
-- Where a scene happens is now its place's anchor in the story
-- (`Location.anchor`, `story::pins`), the one source of pins; the field
-- is gone from the map format, which refuses unknown keys. Every stored
-- whole map loses it: the campaign's maps, the map on the table, and the
-- maps kept as the inverse of a journal line on a map (`map:<id>`).

CREATE FUNCTION pg_temp.labels_without_scenes(map JSONB) RETURNS JSONB AS $$
  SELECT CASE
    WHEN jsonb_typeof(map -> 'labels') = 'array' THEN
      jsonb_set(map, '{labels}', COALESCE(
        (SELECT jsonb_agg(l.value - 'scene' ORDER BY l.ordinality)
           FROM jsonb_array_elements(map -> 'labels') WITH ORDINALITY AS l),
        '[]'::jsonb))
    ELSE map
  END
$$ LANGUAGE sql IMMUTABLE;

UPDATE campaign_maps
   SET map = pg_temp.labels_without_scenes(map)
 WHERE jsonb_path_exists(map, '$.labels[*].scene');

UPDATE map_states
   SET map = pg_temp.labels_without_scenes(map)
 WHERE jsonb_path_exists(map, '$.labels[*].scene');

UPDATE prep_revisions
   SET inverse = pg_temp.labels_without_scenes(inverse)
 WHERE target LIKE 'map:%'
   AND inverse IS NOT NULL
   AND jsonb_path_exists(inverse, '$.labels[*].scene');
