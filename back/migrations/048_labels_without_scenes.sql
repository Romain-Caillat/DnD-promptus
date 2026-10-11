-- campaign/pin-scenes-on-maps — a map's label is only words.
--
-- A label used to name the scene its place opens (`labels[].scene`).
-- Where a scene happens is now its place's anchor in the story
-- (`Location.anchor`, `story::pins`), the one source of pins; the field
-- is gone from the map format, which refuses unknown keys.
--
-- 1. The pins a label gave move to the story, where that can be done
--    without pinning anything else:
--    - a label of a campaign's own map anchors the place of the scene it
--      named on that map's cell, when the place has no anchor yet and no
--      place lies inside it (anchoring a district or a whole system
--      would pin every scene within on that one cell). The first label
--      wins (maps by creation, labels in order);
--    - the world maps compiled into the server named two scenes. Port-Louis
--      on `cotes-bretagne-sud` anchors `lieu_port_louis` there (what the
--      Corsaires file now says) when the scene `sc_port_nuit` stands in
--      it. The Brasier's Carapace (`sc_carapace_sereth`) stood in the
--      whole system (`lieu_brasier`): it is not anchored, the file now
--      gives it a place of its own (`lieu_reliquaire`), reached by
--      re-importing it.
--    A scene with no place, or whose place cannot carry the pin, is left
--    on no map: the validator and the act gauge report it, and the GM
--    pins a place in the Atlas. These writes are not journaled.
-- 2. Every stored whole map loses the field: the campaign's maps, the
--    map on the table, the map of a fight and of a ship battle, and the
--    maps kept as the inverse of a journal line on a map (`map:<id>`).
--
-- Running it twice changes nothing: once stripped, no label names a
-- scene, and an anchor is never overwritten.

CREATE OR REPLACE FUNCTION pg_temp.labels_without_scenes(map JSONB) RETURNS JSONB AS $$
  SELECT CASE
    WHEN jsonb_typeof(map -> 'labels') = 'array' THEN
      jsonb_set(map, '{labels}', COALESCE(
        (SELECT jsonb_agg(l.value - 'scene' ORDER BY l.ordinality)
           FROM jsonb_array_elements(map -> 'labels') WITH ORDINALITY AS l),
        '[]'::jsonb))
    ELSE map
  END
$$ LANGUAGE sql IMMUTABLE;

-- `story` with each place named in `anchors` ({place id: anchor})
-- anchored there, unless it already has an anchor or, with
-- `leaves_only`, a place lies inside it. Order and every other field
-- are kept.
CREATE OR REPLACE FUNCTION pg_temp.anchor_places(story JSONB, anchors JSONB, leaves_only BOOLEAN)
RETURNS JSONB AS $$
  SELECT CASE
    WHEN jsonb_typeof(story -> 'locations') = 'array' THEN
      jsonb_set(story, '{locations}', COALESCE(
        (SELECT jsonb_agg(
                  CASE
                    WHEN anchors ? (l.value ->> 'id')
                         AND NOT l.value ? 'anchor'
                         AND NOT (leaves_only AND EXISTS (
                           SELECT 1 FROM jsonb_array_elements(story -> 'locations') AS inner_place
                            WHERE inner_place ->> 'parent' = l.value ->> 'id'))
                    THEN l.value || jsonb_build_object('anchor', anchors -> (l.value ->> 'id'))
                    ELSE l.value
                  END
                  ORDER BY l.ordinality)
           FROM jsonb_array_elements(story -> 'locations') WITH ORDINALITY AS l),
        '[]'::jsonb))
    ELSE story
  END
$$ LANGUAGE sql IMMUTABLE;

-- 1a. Labels of the campaigns' own maps.
WITH named AS (
  SELECT m.campaign_id,
         COALESCE(m.map ->> 'id', m.id) AS map_id,
         l.value -> 'at' AS at,
         l.value ->> 'scene' AS scene,
         m.created_at, m.id AS map_row, l.ordinality AS ord
    FROM campaign_maps m
   CROSS JOIN LATERAL jsonb_array_elements(
           CASE WHEN jsonb_typeof(m.map -> 'labels') = 'array'
                THEN m.map -> 'labels' ELSE '[]'::jsonb END)
           WITH ORDINALITY AS l
   WHERE l.value ->> 'scene' IS NOT NULL
), placed AS (
  SELECT n.campaign_id, node.value ->> 'location' AS place, n.map_id, n.at,
         row_number() OVER (PARTITION BY n.campaign_id, node.value ->> 'location'
                            ORDER BY n.created_at, n.map_row, n.ord) AS rank
    FROM named n
    JOIN campaigns c ON c.id = n.campaign_id
   CROSS JOIN LATERAL jsonb_array_elements(
           CASE WHEN jsonb_typeof(c.story -> 'nodes') = 'array'
                THEN c.story -> 'nodes' ELSE '[]'::jsonb END) AS node
   WHERE node.value ->> 'id' = n.scene
     AND node.value ->> 'location' IS NOT NULL
), per_campaign AS (
  SELECT campaign_id,
         jsonb_object_agg(place, jsonb_build_object('map', map_id, 'at', at)) AS anchors
    FROM placed
   WHERE rank = 1
   GROUP BY campaign_id
)
UPDATE campaigns c
   SET story = pg_temp.anchor_places(c.story, p.anchors, true)
  FROM per_campaign p
 WHERE p.campaign_id = c.id;

-- 1b. Port-Louis on the Corsaires' world map (`content/maps/corsaires/
-- cotes-bretagne-sud.yaml` named `sc_port_nuit` on [5, 0]), when the
-- scene stands in Port-Louis or a place directly inside it. The city
-- holds the port: on the world map it is the pin of everything played
-- within, as the file now says.
UPDATE campaigns c
   SET story = pg_temp.anchor_places(c.story,
         '{"lieu_port_louis": {"map": "cotes-bretagne-sud", "at": [5, 0]}}'::jsonb, false)
 WHERE c.story -> 'rules' ->> 'id' = 'corsaires'
   AND EXISTS (
     SELECT 1
       FROM jsonb_array_elements(
              CASE WHEN jsonb_typeof(c.story -> 'nodes') = 'array'
                   THEN c.story -> 'nodes' ELSE '[]'::jsonb END) AS node
       LEFT JOIN jsonb_array_elements(
              CASE WHEN jsonb_typeof(c.story -> 'locations') = 'array'
                   THEN c.story -> 'locations' ELSE '[]'::jsonb END) AS place
         ON place ->> 'id' = node ->> 'location'
      WHERE node ->> 'id' = 'sc_port_nuit'
        AND (node ->> 'location' = 'lieu_port_louis'
             OR place ->> 'parent' = 'lieu_port_louis'));

-- 2. Strip the field from every stored copy.
UPDATE campaign_maps
   SET map = pg_temp.labels_without_scenes(map)
 WHERE jsonb_path_exists(map, '$.labels[*].scene');

UPDATE map_states
   SET map = pg_temp.labels_without_scenes(map)
 WHERE jsonb_path_exists(map, '$.labels[*].scene');

UPDATE encounters
   SET fight = jsonb_set(fight, '{map}', pg_temp.labels_without_scenes(fight -> 'map'))
 WHERE jsonb_path_exists(fight, '$.map.labels[*].scene');

UPDATE battles
   SET battle = jsonb_set(battle, '{map}', pg_temp.labels_without_scenes(battle -> 'map'))
 WHERE jsonb_path_exists(battle, '$.map.labels[*].scene');

UPDATE prep_revisions
   SET inverse = pg_temp.labels_without_scenes(inverse)
 WHERE target LIKE 'map:%'
   AND inverse IS NOT NULL
   AND jsonb_path_exists(inverse, '$.labels[*].scene');
