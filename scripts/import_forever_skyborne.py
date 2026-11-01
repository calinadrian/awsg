"""Import local Forever Skyborne characters; source installation stays read-only.

Run with --source, --tools and --listfile as for import_forever_thanes.py.
Uses its CASC reader, the existing character converter and local patch-Y overlay.
Encrypted database blocks are omitted, never decrypted; required rows must exist
in the readable sections. Output is staged before installing the asset overlay.
"""
import sys, runpy, struct, zlib, json, hashlib, re, copy
from pathlib import Path
sys.dont_write_bytecode = True

def readable_table(storage, fid):
    """Preserve offsets while skipping unrelated encrypted BLTE sections."""
    from db2 import Table
    ck = storage.files[fid]
    ek = storage.encoding[ck]
    number, offset, size = storage.index[ek[:9]]
    raw = storage._archive(number)[offset+30:offset+size]
    header = struct.unpack_from('>I', raw, 4)[0]
    if not header:
        return Table(storage.read(fid))
    count = struct.unpack_from('>I', raw, 8)[0] & 0xffffff
    pos, parts = header, []
    for i in range(count):
        size, expanded = struct.unpack_from('>II', raw, 12+i*24)
        block = raw[pos:pos+size]
        pos += size
        if block[:1] == b'E':
            parts.append(bytes(expanded))
        elif block[:1] == b'Z':
            parts.append(zlib.decompress(block[1:]))
        elif block[:1] == b'N':
            parts.append(block[1:])
        else:
            raise ValueError('Unsupported BLTE block')
    return Table(b''.join(parts))

def main():
    if '--out' not in sys.argv:
        sys.argv += ['--out', 'out/forever-skyborne/patch-Y.MPQ']
    base = runpy.run_path(str(Path(__file__).with_name('import_forever_thanes.py')), run_name='skyborne_storage')
    args = base['args']
    from db2 import Table
    import race_assets, retail_textures, blp
    from race_catalog import Race
    from bloodelf_assets import Dbc, dbc_name, RACE_ATLAS
    from durotar_assets import Chain
    from photo_archive import build, read_stored
    a = base['ThanesImporter'](args.source, args.listfile)
    game = Chain(args.data)
    try:
        tables = {k: Table(a.storage.read(fid)) for k,fid in retail_textures.TABLES.items()}
        tables['skinned'] = Table(a.storage.read(3460183))
        chrs = Table(a.storage.read(a.ids['dbfilesclient/chrmodel.db2']))
        links = Table(a.storage.read(a.ids['dbfilesclient/chrracexchrmodel.db2']))
        source_races = Table(a.storage.read(a.ids['dbfilesclient/chrraces.db2']))
        displays = readable_table(a.storage, a.ids['dbfilesclient/creaturedisplayinfo.db2'])
        models = Table(a.storage.read(a.ids['dbfilesclient/creaturemodeldata.db2']))
        info = dict(folder='Skyborne', prefix='Sb', sexes={})
        for sex in (0,1):
            cm, = [r[1] for r in links.rows.values() if r[0] == 95 and r[2] == sex]
            sibling, = [r[1] for r in links.rows.values() if r[0] == 96 and r[2] == sex]
            if sibling != cm:
                raise ValueError('Skyborne variants no longer share character models')
            display = chrs.rows[cm][4]
            model = models.rows[displays.rows[display][1]][2]
            info['sexes'][sex] = dict(model=model, chr_model=cm, texture_layout=chrs.rows[cm][5])
        races = (Race(95,'High Order Skyborne',1,'alliance'), Race(96,'Windshaper Skyborne',2,'horde'))
        classes = {95:[1,3,4,8,11],96:[1,3,4,7,11]}
        combos = Table(a.storage.read(a.ids['dbfilesclient/charbaseinfo.db2']))
        for rid, allowed in classes.items():
            if sorted(r[1] for r in combos.rows.values() if r[0] == rid) != allowed:
                raise ValueError(f'Skyborne class combinations changed for race {rid}')
        dbcs = {n:Dbc(game.read(dbc_name(n))) for n in (*race_assets.TABLE_NAMES,'CreatureDisplayInfoExtra')}
        output = args.out
        output.parent.mkdir(parents=True,exist_ok=True)
        overlay = args.data/'patch-Y.MPQ'
        before = hashlib.sha256(overlay.read_bytes()).hexdigest()
        blobs = read_stored(overlay.read_bytes())
        entries = dict(zip(blobs[0].decode('ascii').splitlines(),blobs[1:],strict=True))
        converted, report = {}, dict(product='wowf',version=a.storage.version,source=str(args.source.resolve()),characters={},races={},base_overlay_sha256=before)
        for sex in (0,1):
            label=('Male','Female')[sex]
            print('Converting Skyborne'+label,flush=True)
            files, character, textures, facial = race_assets.convert(a.storage,game,tables,info,races[0],sex)
            entries.update(files)
            converted[sex]=(character,textures,facial)
            report['characters'][label]=character
            print('Converted',label,'animations',character.get('sequences'),'indices',character.get('indices'),flush=True)
        atlas = blp.decode(game.read(RACE_ATLAS))
        if atlas[:2] != (1024,256):
            raise ValueError('Race portrait atlas layout changed')
        for race, x in zip(races,(640,704)):
            if source_races.string(race.id,2)!=race.name:
                raise ValueError('Source race identity changed')
            own = {}
            for sex,(character,textures,facial) in converted.items():
                tex=copy.copy(textures)
                tex.rows=[(race.id,*row[1:]) for row in textures.rows]
                own[sex]=(character,tex,facial)
            race_assets.add_rows(dbcs,race,info,own)
            base=dbcs['CharBaseInfo'];base.records=[r for r in base.records if r[0]!=race.id]
            for cid in classes[race.id]:base.add_raw(bytes((race.id,cid)))
            # Use the new race's model and class clothing with a freshly composed skin.
            ds=dbcs['CreatureDisplayInfo'];extra=dbcs['CreatureDisplayInfoExtra']
            dr={r[0]:r for r in ds.rows()};er={r[0]:r for r in extra.rows()}
            source=(Path(__file__).resolve().parents[1]/'crates/benilla-app/src/survivors/data.rs').read_text()
            for cid in classes[race.id]:
                block=next(b for b in source.split('ClassDef {')[2:] if re.search(r'id: (\d+)',b)[1]==str(cid))
                looks=[int(x) for x in re.findall(r'\d+',re.search(r'looks: &\[(.*?)\]',block,re.S)[1])]
                template=next(er[dr[d][3]] for d in looks if d in dr and dr[d][3] in er)
                for sex in (0,1):
                    did=300000+cid*200+race.id*2+sex
                    if did in dr or did in er:raise ValueError(f'Display collision {did}')
                    row=list(template);row[:8]=[did,race.id,sex,0,0,0,0,0];row[18]=0;extra.add(row)
                    display=list(dr[200000+race.id*2+sex]);display[0]=display[3]=did;ds.add(display)
            for sex,fid in ((0,8329217),(1,8328923)):
                icon=blp.resize(blp.decode(a.storage.read(fid)),64,64)
                blp.paste(atlas,icon,x,64+sex*128,blend=False)
            report['races'][race.id]=dict(name=race.name,classes=classes[race.id],portrait=[x,64])
        # Replace keys case-insensitively, preserving all unrelated overlay assets.
        updates={dbc_name(n):d.bytes() for n,d in dbcs.items()}
        updates[RACE_ATLAS]=blp.encode(atlas)
        for key,value in updates.items():
            existing=next((k for k in entries if k.lower()==key.lower()),key)
            entries[existing]=value
        game.close()
        archive=build(entries)
        output.write_bytes(archive)
        report['sha256']=hashlib.sha256(archive).hexdigest()
        report['bytes']=len(archive)
        output.with_suffix('.json').write_text(json.dumps(report,indent=2)+'\n')
        print('Staged',str(output),len(archive),flush=True)
    finally:
        game.close();a.storage.close()
if __name__=='__main__':main()
