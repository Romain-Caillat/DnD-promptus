// The campaign reader: Ruby (stdlib YAML) turns a Promptus campaign file
// into the summary the pane draws. Input on stdin: { roots, campaign }.
export const SCAN_RB = String.raw`require 'yaml'
require 'json'
require 'digest'

Encoding.default_external = Encoding::UTF_8
Encoding.default_internal = Encoding::UTF_8

req = JSON.parse($stdin.read)
root = req['roots'].map { |r| File.expand_path(r) }
                   .find { |r| File.directory?(File.join(r, 'content', 'campaigns')) }
unless root
  puts JSON.generate(error: "Aucun projet Promptus trouvé (dossier content/campaigns). Donne son chemin : /promptus <dossier>")
  exit
end

def load(path)
  YAML.safe_load(File.read(path), aliases: true) || {}
rescue StandardError => e
  { '__error' => "#{File.basename(path)} : #{e.message.lines.first.strip}" }
end

campaigns = []
Dir.glob(File.join(root, 'content', 'campaigns', '*', '*.y{a,}ml')).sort.each do |p|
  campaigns << { key: File.basename(File.dirname(p)), path: p, isExample: false }
end
Dir.glob(File.join(root, 'content', 'fixtures', '*.y{a,}ml')).sort.each do |p|
  campaigns << { key: File.basename(p, '.*'), path: p, isExample: true }
end
campaigns.each do |c|
  head = File.foreach(c[:path]).first(80).find { |l| l.start_with?('title:') }
  c[:title] = head ? head.sub('title:', '').strip.delete('"\'') : c[:key]
end

wanted = req['campaign'].to_s.downcase
current = campaigns.find { |c| c[:key].downcase == wanted } ||
          campaigns.find { |c| !wanted.empty? && "#{c[:key]} #{c[:title]}".downcase.include?(wanted) } ||
          campaigns.find { |c| !c[:isExample] } || campaigns.first
out = { root: root, campaigns: campaigns, key: '', title: '', path: '', mtimeMs: 0, acts: [], error: '' }
unless current
  out[:error] = 'Aucune campagne dans content/campaigns ni content/fixtures.'
  puts JSON.generate(out)
  exit
end

doc = load(current[:path])
out.merge!(key: current[:key], title: doc['title'] || current[:title], path: current[:path],
           mtimeMs: (File.mtime(current[:path]).to_f * 1000).to_i)
if doc['__error']
  out[:error] = doc['__error']
  puts JSON.generate(out)
  exit
end

map_ids = {}
Dir.glob(File.join(root, 'content', 'maps', '**', '*.y{a,}ml')).each do |p|
  id = File.foreach(p).find { |l| l.start_with?('id:') }
  map_ids[id.sub('id:', '').strip.delete('"\'')] = true if id
end

by_id = ->(key) { (doc[key] || []).to_h { |e| [e['id'], e] } }
locations = by_id.call('locations')
npcs = by_id.call('npcs')
nodes = doc['nodes'] || []
titles = nodes.to_h { |n| [n['id'], n['title'] || n['id']] }
blank = ->(v) { v.nil? || (v.respond_to?(:empty?) && v.empty?) }

scenes = nodes.map do |n|
  amb = n['ambience'] || {}
  music = amb['music'] || []
  exits = n['exits'] || []
  missing = []
  missing << 'résumé' if blank.call(n['summary'])
  missing << 'lieu' if blank.call(n['location'])
  missing << 'ambiance' if blank.call(amb['mood'])
  missing << "texte d'entrée" if blank.call(n['read_aloud'])
  missing << 'déroulé' if blank.call(n['flow'])
  missing << 'transition' if !exits.empty? && blank.call(n['transition'])
  cast = (n['npcs'] || []).map { |s| npcs[s['npc']] }.compact
  loc = locations[n['location']]
  {
    id: n['id'], act: n['act'].to_s, title: n['title'] || n['id'], optional: n['optional'] == true,
    summary: n['summary'].to_s.strip, location: loc ? loc['name'].to_s : n['location'].to_s,
    hook: n['hook'].to_s.strip, exits: exits.map { |x| titles[x['to']] || x['to'] },
    mood: amb['mood'].to_s, invented: JSON.generate(n).include?('INVENTÉ'), missing: missing,
    map: n['map'] ? { id: n['map'], isFound: map_ids.key?(n['map']) } : nil,
    music: { chosen: music.count { |m| !blank.call(m['url']) }, toFind: music.count { |m| blank.call(m['url']) } },
    hasArt: !blank.call(n['art']),
    portraits: { described: cast.count { |c| !blank.call(c['portrait']) }, total: cast.size },
    opponents: ((n['encounter'] || {})['opponents'] || []).sum { |o| (o['count'] || 1).to_i },
    hash: Digest::SHA1.hexdigest(JSON.generate(n))[0, 12]
  }
end

acts = (doc['acts'] || []).map { |a| { id: a['id'].to_s, title: a['title'] || a['id'], scenes: [] } }
scenes.each do |s|
  act = acts.find { |a| a[:id] == s[:act] }
  act ||= (acts << { id: s[:act], title: s[:act].empty? ? 'Sans acte' : s[:act], scenes: [] }).last
  act[:scenes] << s
end
out[:acts] = acts
puts JSON.generate(out)
`
