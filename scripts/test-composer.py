#!/usr/bin/env python3
"""Review the index, adjust staged files and write a multiline commit in the real TUI."""
import os
import runpy
import tempfile
from pathlib import Path
w=runpy.run_path(str(Path(__file__).with_name('test-workspace.py')))
Tui,git,init=(w[n] for n in ('Tui','git','init'))


def run():
    with tempfile.TemporaryDirectory(prefix='lazykeifu-compose-') as temp:
        base=Path(temp); root=base/'project'; root.mkdir(); hooks=init(root)
        for name in ('a','b'): (root/name).write_text('base\n')
        git(root,'add','.'); git(root,'commit','-m','base')
        original=git(root,'rev-parse','HEAD').stdout.strip()
        (root/'a').write_text('staged-a\n'); (root/'b').write_text('staged-b\n');git(root,'add','a','b')
        (root/'a').write_text('worktree-only-a\n')
        config=base/'config';(config/'lazykeifu').mkdir(parents=True)
        (config/'lazykeifu/config.toml').write_text('[refresh]\nauto_fetch=false\nauto_refresh=false\n')
        tui=Tui(root,config,stdin_pipe=True)
        try:
            tui.keys('c'); tui.loaded(); screen=tui.screen()
            assert 'Staged · 2 files' in screen and 'Body (optional)' in screen,screen
            assert '+staged-a' in screen and '+worktree-only-a' not in screen,screen
            assert '4 Graph' in screen
            # Paste through the real PTY, including newlines: it must never submit.
            message='日本語の件名\n\n理由その一\n理由その二'
            tui.proc.stdin.write(('\x1b[200~'+message+'\x1b[201~').encode());tui.proc.stdin.flush()
            tui.wait(lambda:tui.state()['composer']['message']==message)
            assert git(root,'rev-parse','HEAD').stdout.strip()==original
            assert tui.state()['composer']['field']=='Body'
            tui.keys('<enter> <backspace>')
            assert tui.state()['composer']['message']==message
            assert git(root,'rev-parse','HEAD').stdout.strip()==original
            tui.keys('<esc> c');tui.loaded()
            assert tui.state()['composer']['message']==message
            tui.keys('<backtab> j');tui.loaded()
            assert '+staged-b' in tui.screen()
            tui.keys('<space>');tui.loaded()
            assert tui.state()['composer']['staged_count']==1
            assert git(root,'diff','--cached','--name-only').stdout.strip()=='a'
            assert (root/'b').read_text()=='staged-b\n'
            assert '+staged-a' in tui.screen()
            # Mouse file selection and editor placement use the visible fields.
            screen=tui.screen(); x,y,_,_=tui.state()['composer']['fields'][0]
            tui.request(cmd='mouse',kind='click',x=x+1,y=y+1)
            assert tui.state()['composer']['field']=='Subject'
            hook=hooks/'pre-commit';hook.write_text('#!/bin/sh\necho rejected-test >&2\n' + ''.join(f'echo output-{i} >&2\n' for i in range(15)) + 'echo output-end >&2\nexit 1\n');hook.chmod(0o755)
            tui.keys('<c-s>');tui.wait(lambda:not tui.state()['workspace_busy'])
            assert tui.state()['mode']=='input'
            assert tui.state()['composer']['message']==message
            assert 'rejected-test' in tui.screen(),tui.screen()
            tui.proc.stdin.write(b'\x1bj'*25);tui.proc.stdin.flush()
            tui.wait(lambda:'output-end' in tui.screen())
            assert git(root,'rev-parse','HEAD').stdout.strip()==original
            hook.write_text('#!/bin/sh\nexit 0\n')
            # External changes to the index require reviewing the new snapshot.
            git(root,'add','b');tui.keys('<c-s>')
            assert git(root,'rev-parse','HEAD').stdout.strip()==original
            assert tui.state()['composer']['staged_count']==2
            assert 'Staged changes or HEAD changed' in tui.screen(),tui.screen()
            tui.keys('<c-s>');tui.wait(lambda:not tui.state()['workspace_busy'])
            assert tui.state()['mode']=='normal'
            assert git(root,'log','-1','--format=%B').stdout.strip()==message
            assert git(root,'show','HEAD:a').stdout=='staged-a\n'
            assert (root/'a').read_text()=='worktree-only-a\n'
            assert tui.state()['composer']['message']==''
            # Long drafts and compact viewports remain navigable; remove last file safely.
            git(root,'add','a');tui.keys('c');tui.loaded()
            tui.request(cmd='paste',text='Long subject '+('語'*80)+'\n\n'+ '\n'.join(f'body {i}' for i in range(20)))
            for width,height in ((120,32),(80,24),(54,20)):
                screen=tui.screen(width,height)
                assert '4 Graph' in screen and 'STAGED' in screen,screen
                if os.environ.get('LAZYKEIFU_TEST_ARTIFACTS'):
                    dest=Path(os.environ['LAZYKEIFU_TEST_ARTIFACTS']);dest.mkdir(parents=True,exist_ok=True)
                    (dest/f'composer-{width}.txt').write_text(screen)
            tui.keys('<tab> <space>');tui.loaded()
            assert tui.state()['composer']['staged_count']==0
            tui.keys('<c-s>')
            assert tui.state()['mode']=='input'
            assert 'No staged changes' in tui.screen()
            tui.keys('<esc> c')
            assert tui.state()['mode']=='normal'
            assert tui.state()['composer']['message'].startswith('Long subject')
        finally:tui.close()
    print('PASS: staged-only partial diff, review/unstage, draft resume, native multiline paste, hook retry, changed index review, commit contents, compact editing')


if __name__=='__main__':run()
