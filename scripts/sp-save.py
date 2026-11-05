"""Read or edit the ORIGINAL game's single-player save (characters.txt) outside the game.

The save is encrypted (RSE1 container, key embedded in the game exe) and lives at
%APPDATA%\\wenilla-rogue\\characters.txt. It is plain text once decrypted: one
pipe-delimited record per line, and the per-class talent experience is the record

    talent|<class>|<xp>|<key:rank,key:rank,...>        e.g.  talent|4|500|imp_sinister:3

Talent XP is what every finished run pays its class (time survived, kills, bosses,
guardians, a win, scaled by zone); accumulated XP buys points, the nth costing
100 + 25*n, at most 51. Points are spent in the armory (the talent screen).
This tool lets you grant that XP without playing: `add`/`set` change the xp field,
keeping everything else byte-identical (the original nonce is preserved, so an
unchanged save reseals to the exact same file).

`import` bridges the two saves: the reimplementation (benilla) keeps its own talent
book, benilla-config/survivors-talents.txt, one class a line:

    class xp key=rank key=rank ...            e.g.  7 12960 convection=5 concussion=5

Its xp is CUMULATIVE (points are bought out of it, 100+25n each), while the original
save's xp is the balance left after spending. `import` therefore writes
    book xp = save xp + cost of the ranks the save already spent
so the reimplementation shows the same ranks with the same points still to spend.
Keys are the same names in both games; a key the reimplementation lacks is ignored
by its loader. Close the reimplementation before importing (it re-saves on exit).

Usage:
  sp-save.py [show]                 show the decrypted save (default)
  sp-save.py points [CLASS]         points earned per class (or one class)
  sp-save.py add CLASS AMOUNT       add AMOUNT talent xp to a class
  sp-save.py set CLASS XP           set a class's talent xp exactly
  sp-save.py classes                list class ids and names
  sp-save.py import [CLASS]         copy the save's talent xp+ranks into the reimplementation's book

Options:
  --exe PATH      game exe the key is extracted from (default: the installed ASR.exe)
  --save PATH     the save (default: %APPDATA%\\wenilla-rogue\\characters.txt)
  --recon PATH    folder holding azs_rse1.py (the reference RSE1 implementation)
  --book PATH     the reimplementation's talent book (import only; default:
                  BENILLA_HOME, then the installed ASR Game config if present,
                  then <repo>/benilla-config/survivors-talents.txt)
  --dry-run       with import, show what would be written without writing

Writes back atomically: backs up first, refuses if the file changed on disk since
it was read, re-verifies the resealed save opens, and preserves the nonce.
Close the game before using it.
"""
import argparse
import os
import re
import sys
import time
from pathlib import Path

DEFAULT_EXES = [
    r'D:\Personal\ASR\ASR.exe',
    r'C:\Users\hasyl\AppData\Local\WorldOfSurvive\game\AzerothSurvivors.exe',
]
DEFAULT_RECONS = [
    r'C:\Users\hasyl\AppData\Local\WorldOfSurvive\game\reconstruction',
]

# The game writes records in this order (azs_savecompat.WRITE_ORDER); a new talent
# line is inserted at its place in it rather than tacked on.
WRITE_ORDER = ('pick', 'unlock', 'open', 'items', 'equip', 'look', 'talent', 'seen',
               'ach', 'achat', 'stat', 'beat', 'perk', 'tabard', 'pets', 'resets', 'hero')

CLASSES = {1: 'Warrior', 2: 'Paladin', 3: 'Hunter', 4: 'Rogue', 5: 'Priest',
           6: 'Death Knight', 7: 'Shaman', 8: 'Mage', 9: 'Warlock', 11: 'Druid'}
CLASS_BY_NAME = {v.lower(): k for k, v in CLASSES.items()}

MAX_POINTS = 51


def point_cost(n):
    return 100 + 25 * n


def earned_points(xp):
    n, left = 0, xp
    while n < MAX_POINTS and left >= point_cost(n):
        left -= point_cost(n)
        n += 1
    return n, left


def import_rse1(recon):
    if recon:
        sys.path.insert(0, str(recon))
    else:
        for d in DEFAULT_RECONS:
            if (Path(d) / 'azs_rse1.py').is_file():
                sys.path.insert(0, d)
                break
    try:
        import azs_rse1
    except ImportError:
        sys.exit('Could not import azs_rse1; pass --recon <folder with azs_rse1.py>.')
    return azs_rse1


def resolve(args):
    exe = Path(args.exe) if args.exe else next((Path(p) for p in DEFAULT_EXES if Path(p).is_file()), None)
    if exe is None:
        sys.exit('No game exe found; pass --exe PATH.')
    save = Path(os.path.expandvars(args.save)) if args.save else \
        Path(os.environ['APPDATA']) / 'wenilla-rogue' / 'characters.txt'
    if not save.is_file():
        sys.exit('Save not found: %s (pass --save PATH)' % save)
    return exe, save


def load(args):
    rse1 = import_rse1(args.recon)
    exe, save = resolve(args)
    original = save.read_bytes()
    try:
        key = rse1.key_from_exe(str(exe), original)
    except (rse1.Rse1Error, OSError) as error:
        sys.exit('Could not open the save (%s).' % error)
    nonce, text = rse1.open_save(original, key)
    return rse1, exe, save, original, nonce, key, text


def class_id(name):
    if name.isdigit():
        if int(name) not in CLASSES:
            sys.exit('Unknown class id %s (use `classes`).' % name)
        return int(name)
    cid = CLASS_BY_NAME.get(name.lower())
    if cid is None:
        sys.exit('Unknown class %r (use `classes`).' % name)
    return cid


def talent_lines(text):
    out = []
    for i, line in enumerate(text.splitlines()):
        f = line.split('|')
        if f and f[0] == 'talent':
            out.append((i, f))
    return out


def fmt_xp(value, original):
    if '.' not in original:
        return str(int(round(value)))
    return ('%g' % value) if value == int(value) else repr(value)


def spent_cost(ranks):
    # Buying `ranks` points costs point_cost(n) for n = 0..ranks-1, i.e. 100n + 25n(n-1)/2.
    return 100 * ranks + 25 * ranks * (ranks - 1) // 2


def book_default():
    if os.environ.get('BENILLA_HOME'):
        return Path(os.environ['BENILLA_HOME']) / 'survivors-talents.txt'
    if os.environ.get('LOCALAPPDATA'):
        installed = Path(os.environ['LOCALAPPDATA']) / 'ASRGame' / 'benilla-config'
        if installed.is_dir():
            return installed / 'survivors-talents.txt'
    return Path(__file__).resolve().parent.parent / 'benilla-config' / 'survivors-talents.txt'


def parse_book(text):
    book = {}
    for line in text.splitlines():
        w = line.split()
        if len(w) < 2:
            continue
        try:
            cid, xp = int(w[0]), float(w[1])
        except ValueError:
            continue
        specs = [[], [], []]
        active = 0
        for tok in w[2:]:
            if tok.startswith('active='):
                try:
                    active = min(2, max(0, int(tok[7:])))
                except ValueError:
                    pass
                continue
            spec, sep, entries = tok.partition(':')
            if sep and spec in ('S0', 'S1', 'S2'):
                for entry in entries.split(','):
                    k, eq, r = entry.partition('=')
                    if eq and r.isdigit():
                        specs[int(spec[1])].append((k, int(r)))
                continue
            k, sep, r = tok.partition('=')
            if sep and r.isdigit():
                specs[0].append((k, int(r)))
        book[cid] = (xp, specs, active)
    return book


def render_book(book):
    # Preserve all saved specs when importing any class from the original game.
    lines = []
    for cid, (xp, specs, active) in sorted(book.items()):
        xp_s = str(int(xp)) if xp == int(xp) else repr(xp)
        slots = ' '.join('S%d:%s' % (i, ','.join('%s=%d' % (k, r) for k, r in sorted(ranks)))
                         for i, ranks in enumerate(specs))
        lines.append('%d %s active=%d %s' % (cid, xp_s, active, slots))
    lines.sort()
    return '\n'.join(lines) + '\n'


NOTE = ('  (points estimated with the 100+25n model: the nth point costs 100+25n xp, max 51; '
        'the game keeps its own conversion. Where a class has spent more ranks than the xp '
        'field yields, the field is most likely a balance the game decrements as points are spent.)')


def show_save(text):
    print(text)
    print()
    print('Talent lines:')
    for i, f in talent_lines(text):
        cid, xp = int(f[1]), float(f[2])
        pts, into = earned_points(xp)
        spent = sum(int(r) for r in re.findall(r':(\d+)', f[3] if len(f) > 3 else ''))
        print('  line %d: talent|%s|%s  ->  %s: ~%d points (%d spent, %d xp into the next)'
              % (i + 1, f[1], f[2], CLASSES.get(cid, '?'), pts, spent, into))
    print(NOTE)


def show_points(text, only):
    rows = talent_lines(text)
    if not rows:
        print('No talent lines in the save (no class has earned talent xp yet).')
        return
    for i, f in rows:
        cid = int(f[1])
        if only is not None and cid != only:
            continue
        xp = float(f[2])
        pts, into = earned_points(xp)
        next_cost = point_cost(pts) if pts < MAX_POINTS else 0
        spent = sum(int(r) for r in re.findall(r':(\d+)', f[3] if len(f) > 3 else ''))
        print('%-13s (id %d): xp %d  ->  %d points, %d spent, %d unspent; %d xp into the next (cost %d)%s'
              % (CLASSES.get(cid, '?'), cid, xp, pts, spent, pts - spent, into, next_cost,
                 '  [MAX]' if pts >= MAX_POINTS else ''))


def edit_save(text, cid, new_xp, original):
    rows = talent_lines(text)
    mine = [r for r in rows if int(r[1][1]) == cid]
    if len(mine) > 1:
        sys.exit('Multiple talent lines for class %d; refusing to guess.' % cid)
    lines = text.splitlines()
    if mine:
        i, f = mine[0]
        f[2] = fmt_xp(new_xp, f[2])
        lines[i] = '|'.join(f)
    else:
        line = 'talent|%d|%s|' % (cid, fmt_xp(new_xp, '0'))
        insert = len(lines)
        for j, l in enumerate(lines):
            kind = l.split('|')[0]
            if kind in WRITE_ORDER and WRITE_ORDER.index(kind) > WRITE_ORDER.index('talent'):
                insert = j
                break
        lines.insert(insert, line)
    return '\n'.join(lines) + '\n'


def commit(args, rse1, save, original, nonce, key, new_text):
    backup = save.with_name('characters.bak-%s.txt' % time.strftime('%Y%m%d-%H%M%S'))
    backup.write_bytes(original)
    sealed = rse1.seal(new_text, key, nonce)
    # Re-verify what we are about to write.
    n2, t2 = rse1.open_save(sealed, key)
    assert n2 == nonce and t2 == new_text, 'Resealed save did not round-trip; nothing written.'
    if save.read_bytes() != original:
        sys.exit('The save changed on disk since it was read (is the game open?); nothing written.')
    tmp = save.with_name('characters.sp.tmp')
    tmp.write_bytes(sealed)
    tmp.replace(save)
    print('Backup: %s' % backup.name)
    print('Wrote %s (%d bytes)' % (save, len(sealed)))


def import_book(args, text, only):
    rows = [r for r in talent_lines(text) if only is None or int(r[1][1]) == only]
    if not rows:
        sys.exit('No talent lines to import%s.' % (' for class %d' % only if only else ''))
    book_path = Path(args.book) if args.book else book_default()
    book = parse_book(book_path.read_text(encoding='utf-8')) if book_path.is_file() else {}
    for i, f in rows:
        cid, xp0 = int(f[1]), float(f[2])
        ranks = []
        for tok in (f[3].split(',') if len(f) > 3 and f[3] else []):
            k, sep, r = tok.partition(':')
            if sep and r.isdigit():
                ranks.append((k, int(r)))
        spent = sum(r for _, r in ranks)
        xp1 = xp0 + spent_cost(spent)
        _, specs, active = book.get(cid, (0, [[], [], []], 0))
        specs[0] = sorted(ranks)
        book[cid] = (xp1, specs, active)
        pts, _ = earned_points(xp1)
        print('%s: save balance %d + spent cost %d (%d ranks) -> book xp %d  =>  %d earned, '
              '%d spent, %d unspent%s'
              % (CLASSES.get(cid, '?'), int(xp0), spent_cost(spent), spent, int(xp1), pts, spent,
                 pts - spent, '  [ranks above the 51-point cap]' if spent > pts else ''))
    if args.dry_run:
        print('\n--dry-run: %s would become:\n%s' % (book_path, render_book(book)))
        return
    if book_path.is_file():
        backup = book_path.with_name(book_path.name + '.bak-' + time.strftime('%Y%m%d-%H%M%S'))
        backup.write_text(book_path.read_text(encoding='utf-8'), encoding='utf-8')
        print('Backup: %s' % backup.name)
    else:
        book_path.parent.mkdir(parents=True, exist_ok=True)
    tmp = book_path.with_name(book_path.name + '.sp.tmp')
    tmp.write_text(render_book(book), encoding='utf-8')
    tmp.replace(book_path)
    print('Wrote %s' % book_path)
    print('Close the reimplementation before this, and restart it to see the talents.')


def main(argv=None):
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument('command', nargs='?', default='show',
                    choices=['show', 'points', 'add', 'set', 'classes', 'import'])
    ap.add_argument('args', nargs='*', metavar='ARG')
    ap.add_argument('--exe', help='game exe (for the save key)')
    ap.add_argument('--save', help='path to characters.txt')
    ap.add_argument('--recon', help='folder holding azs_rse1.py')
    ap.add_argument('--book', help='reimplementation talent book (import only)')
    ap.add_argument('--dry-run', action='store_true', help='import: show, do not write')
    a = ap.parse_args(argv)

    if a.command == 'classes':
        for cid in sorted(CLASSES):
            print('%2d  %s' % (cid, CLASSES[cid]))
        return 0

    rse1, exe, save, original, nonce, key, text = load(a)
    if a.command == 'show':
        show_save(text)
        return 0
    if a.command == 'points':
        only = class_id(a.args[0]) if a.args else None
        show_points(text, only)
        return 0
    if a.command == 'import':
        only = class_id(a.args[0]) if a.args else None
        import_book(a, text, only)
        return 0

    if not a.args:
        sys.exit('%s needs CLASS and %s.' % (a.command, 'AMOUNT' if a.command == 'add' else 'XP'))
    cid = class_id(a.args[0])
    amount = int(a.args[1])
    if a.command == 'add' and amount < 0:
        sys.exit('AMOUNT must be >= 0 (use set to lower it).')
    if a.command == 'set' and amount < 0:
        sys.exit('XP must be >= 0.')
    before = [int(r[1][1]) for r in talent_lines(text)]
    xp0 = float([r for r in talent_lines(text) if int(r[1][1]) == cid][0][1][2]) if cid in before else 0.0
    xp1 = xp0 + amount if a.command == 'add' else amount
    new_text = edit_save(text, cid, xp1, original)
    p0, _ = earned_points(xp0)
    p1, _ = earned_points(xp1)
    print('%s %s: %d -> %d talent xp  (%d -> %d points)%s'
          % (a.command.upper(), CLASSES[cid], xp0, int(xp1), p0, p1,
             '' if p1 > p0 else '  [no new point]'))
    if a.command in ('add', 'set'):
        commit(a, rse1, save, original, nonce, key, new_text)
    return 0


if __name__ == '__main__':
    sys.exit(main())
