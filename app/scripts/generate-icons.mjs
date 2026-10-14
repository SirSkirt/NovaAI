#!/usr/bin/env node
/**
 * Regenerate every standard Tauri icon from the ORIGINAL user-provided
 * app/src-tauri/icons/icon.png. A temporary copy avoids input/output
 * collisions because Tauri also writes a generated file named icon.png.
 *
 * To preserve the original when the CLI is run twice during Android CI,
 * set NOVA_ICON_SOURCE to a copy saved in RUNNER_TEMP before first generation.
 * This changes ONLY packaging icons, not the neural model or visualizer.
 */
import {readFileSync,copyFileSync,mkdtempSync,rmSync} from 'node:fs';
import {join,resolve,dirname} from 'node:path';
import {tmpdir} from 'node:os';
import {fileURLToPath} from 'node:url';
import {spawnSync} from 'node:child_process';

const app=resolve(dirname(fileURLToPath(import.meta.url)),'..');
const source=process.env.NOVA_ICON_SOURCE
    ?resolve(process.env.NOVA_ICON_SOURCE)
    :join(app,'src-tauri','icons','icon.png');
const content=readFileSync(source);
if(content.length<24 || content.subarray(0,8).toString('hex')!=='89504e470d0a1a0a'){
  throw Error('Expected an authentic PNG image at '+source);
}
const width=content.readUInt32BE(16),height=content.readUInt32BE(20);
if(width!==height || width<512){
  throw Error('Neural Nova source icon must be square and >=512px, got '+width+'x'+height);
}
const temp=mkdtempSync(join(tmpdir(),'nova-tauri-icon-'));
try{
  const copy=join(temp,'neural-nova-icon.png');
  copyFileSync(source,copy);
  console.log('Generating Tauri icon set from '+source+' ('+width+'x'+height+')');
  // The "tauri" npm script resolves the installed @tauri-apps/cli.
  // Explicit -- passes "icon" and its PNG path through to Tauri.
  const p=spawnSync('npm',['run','tauri','--','icon',copy],{
    cwd:app,stdio:'inherit',shell:process.platform==='win32'
  });
  if(p.error)throw p.error;
  if(p.status!==0)throw Error('Tauri icon generation failed with status '+p.status);
  console.log('Tauri icon generation completed successfully.');
}finally{
  rmSync(temp,{recursive:true,force:true});
}
