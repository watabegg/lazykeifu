#!/usr/bin/env python3
"""Real TUI: amend/reword, reset/reflog, tags and branch rename."""
import runpy
import tempfile
from pathlib import Path
w=runpy.run_path(str(Path(__file__).with_name('test-workspace.py')))
Tui,git,init=(w[n] for n in ('Tui','git','init'))

def choose(tui,label):
    menu=tui.state()['git_menu']
    assert menu, tui.screen()
    i=next(i for i,v in enumerate(menu['items']) if v.startswith(label))
    tui.keys('<home> '+'j '*i+'<enter>')

def run():
    with tempfile.TemporaryDirectory(prefix='pedigit-history-edit-') as temp:
        base=Path(temp);root=base/'repo';root.mkdir();hooks=init(root)
        file=root/'file';file.write_text('base\n');git(root,'add','.');git(root,'commit','-m','base')
        parent=git(root,'rev-parse','HEAD').stdout.strip()
        file.write_text('second\n');git(root,'add','.');git(root,'commit','-m','second')
        original=git(root,'rev-parse','HEAD').stdout.strip()
        config=base/'config';(config/'pedigit').mkdir(parents=True)
        (config/'pedigit/config.toml').write_text('[refresh]\nauto_fetch=false\nauto_refresh=false\n')
        file.write_text('staged\n');git(root,'add','file');file.write_text('unstaged\n')
        tui=Tui(root,config)
        try:
            tui.keys('@');assert 'Amend HEAD' in tui.screen();choose(tui,'Amend HEAD')
            assert 'Published' in tui.screen(),tui.screen()
            tui.keys('<esc>');assert git(root,'rev-parse','HEAD').stdout.strip()==original
            tui.keys('@');choose(tui,'Amend HEAD');tui.keys('<enter>');tui.loaded()
            assert 'Amend HEAD' in tui.screen() and '+staged' in tui.screen(),tui.screen()
            tui.keys('<home>');tui.request(cmd='paste',text='Amended ')
            # Hook failure must retain amend mode and edited draft.
            hook=hooks/'pre-commit';hook.write_text('#!/bin/sh\necho amend-rejected >&2\nexit 1\n');hook.chmod(0o755)
            tui.keys('<c-s>');tui.wait(lambda:not tui.state()['workspace_busy'])
            assert 'amend-rejected' in tui.screen() and tui.state()['mode']=='input'
            assert git(root,'rev-parse','HEAD').stdout.strip()==original
            hook.unlink();tui.keys('<c-s>');tui.wait(lambda:not tui.state()['workspace_busy'])
            assert tui.state()['mode']=='normal',tui.screen()
            assert git(root,'rev-parse','HEAD^').stdout.strip()==parent
            assert git(root,'show','HEAD:file').stdout=='staged\n' and file.read_text()=='unstaged\n'
            assert git(root,'log','-1','--format=%s').stdout.strip()=='Amended second'
            git(root,'add','file');index=git(root,'write-tree').stdout
            tui.keys('@');choose(tui,'Reword HEAD');tui.keys('<enter>');tui.loaded()
            assert 'Message-only amend' in tui.screen(),tui.screen()
            tui.keys('<home>');tui.request(cmd='paste',text='Reword ');tui.keys('<c-s>');tui.wait(lambda:not tui.state()['workspace_busy'])
            amended=git(root,'rev-parse','HEAD').stdout.strip()
            assert git(root,'write-tree').stdout==index and git(root,'show','HEAD:file').stdout=='staged\n'
            tui.keys('@');choose(tui,'Undo last');tui.keys('<enter>');tui.wait(lambda:not tui.state()['workspace_busy'])
            assert git(root,'rev-parse','HEAD').stdout.strip()==parent and file.read_text()=='unstaged\n'
            assert git(root,'write-tree').stdout==index
            # Reflog selects exact OID; soft restoration preserves files and index.
            tui.keys('@');choose(tui,'Reflog');assert amended[:7] in tui.screen(),tui.screen()
            menu=tui.state()['git_menu'];label=next(v for v in menu['items'] if amended[:7] in v)
            choose(tui,label);choose(tui,'--soft');tui.keys('<enter>');tui.wait(lambda:not tui.state()['workspace_busy'])
            assert git(root,'rev-parse','HEAD').stdout.strip()==amended and git(root,'write-tree').stdout==index
            tui.keys('2 @');choose(tui,'Reset current');choose(tui,'--mixed');tui.keys('<enter>');tui.wait(lambda:not tui.state()['workspace_busy'])
            assert not git(root,'diff','--cached','--name-only').stdout.strip() and file.read_text()=='unstaged\n'
            tui.keys('@');choose(tui,'Reset current');choose(tui,'--hard');tui.keys('<enter>');tui.wait(lambda:not tui.state()['workspace_busy'])
            assert 'clean worktree' in tui.screen(),tui.screen();assert file.read_text()=='unstaged\n'
            tui.keys('<esc> @');choose(tui,'Create tag');tui.keys('v 1 <enter>')
            assert git(root,'rev-parse','v1').stdout.strip()==amended
            tui.keys('3 @');choose(tui,'Rename branch');tui.keys('<backspace> <backspace> <backspace> <backspace> t o p i c <enter>')
            assert git(root,'branch','--show-current').stdout.strip()=='topic'
            # A stale confirmation cannot reset a newly moved HEAD.
            tui.keys('@');choose(tui,'Undo last');git(root,'add','file');git(root,'commit','-m','external')
            fresh=git(root,'rev-parse','HEAD').stdout.strip();tui.keys('<enter>');tui.wait(lambda:not tui.state()['workspace_busy'])
            assert 'changed' in tui.screen() and git(root,'rev-parse','HEAD').stdout.strip()==fresh
        finally:tui.close()
    print('PASS: amend cancel/hook retry, message-only amend preserves index, undo, reflog restore, mixed reset, dirty hard reset refusal, tag, rename, stale reset')

if __name__=='__main__':run()
