#!/usr/bin/env python3
"""Drive the real diff renderer, changed-file statistics and tree status indentation."""
import runpy
import tempfile
from pathlib import Path
w=runpy.run_path(str(Path(__file__).with_name('test-workspace.py')))
Tui,git,init=(w[n] for n in ('Tui','git','init'))

def run():
    with tempfile.TemporaryDirectory(prefix='lazykeifu-diff-polish-') as temp:
        base=Path(temp);root=base/'repo';root.mkdir();init(root);(root/'src').mkdir()
        file=root/'src/main.rs';file.write_text('fn main() {\n    println!("old");\n}\n')
        git(root,'add','.');git(root,'commit','-m','base')
        file.write_text('fn main() {\n    println!("new");\n}\n');git(root,'commit','-am','change greeting')
        file.write_text('fn main() {\n    println!("working");\n    println!("extra");\n}\n')
        config=base/'config';(config/'lazykeifu').mkdir(parents=True)
        (config/'lazykeifu/config.toml').write_text('[refresh]\nauto_fetch=false\nauto_refresh=false\n')
        tui=Tui(root,config)
        try:
            tui.loaded();screen=tui.screen()
            directory=next(line for line in screen.splitlines() if '▾ src/' in line)
            leaf=next(line for line in screen.splitlines() if 'main.rs' in line and ' M ' in line)
            assert leaf.index('M')>directory.index('▾'),screen
            tui.keys('<enter>');tui.loaded();screen=tui.screen()
            assert '+2  -1' in screen and 'src/main.rs' in screen,screen
            assert 'diff --git' not in screen and 'index ' not in screen,screen
            assert '   2' in screen and '+    println!("working")' in screen,screen
            evidence=Path('/home/watabegg/.cache/lazykeifu/history-ui-evidence');evidence.mkdir(exist_ok=True)
            (evidence/'diff-after-120.txt').write_text(screen)
            for width,height in ((80,24),(54,20)):
                screen=tui.screen(width,height)
                assert 'STAGED' in screen and '+2  -1' in screen,screen
                (evidence/f'diff-after-{width}.txt').write_text(screen)
            tui.keys('<esc> 4 <home>');tui.wait(lambda:'+2  -1' in tui.screen())
            assert 'Uncommitted Changes' in tui.screen() and 'Changed files' in tui.screen(),tui.screen()
            (evidence/'uncommitted-detail.txt').write_text(tui.screen())
            tui.keys('j');tui.wait(lambda:'+1  -1' in tui.screen())
            assert 'Commit Detail' in tui.screen() and '1 files' in tui.screen(),tui.screen()
            tui.keys('6 <end>');assert 'src/main.rs' in tui.screen()
            (evidence/'commit-detail.txt').write_text(tui.screen())
            tui.keys('@');screen=tui.screen(54,20);assert 'Extras' in screen
            tui.screen();tui.request(cmd='mouse',kind='click',x=119,y=0)
            assert tui.state()['mode']=='normal'
        finally:tui.close()
    print('PASS: status inside tree, line numbers and clean patch, +/- totals for commit/uncommitted/file, narrow diff and menu dismissal')

if __name__=='__main__':run()
