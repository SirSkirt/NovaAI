"use strict";
const assert=require("node:assert/strict");
const fs=require("node:fs");
const path=require("node:path");
const vm=require("node:vm");
const script=fs.readFileSync(path.join(__dirname,"../dist/js/checkpoint-ui.js"),"utf8");

function mock(userAgent="Android"){
  const elements=new Map();
  const dom=id=>{
    if(!elements.has(id)){
      const listeners={};
      elements.set(id,{
        id,listeners,files:[],disabled:false,hidden:false,
        style:{display:""},textContent:"",href:"",download:"",
        addEventListener(name,handler){listeners[name]=handler;},
        click(){if(this.listeners.click)return this.listeners.click();},
      });
    }
    return elements.get(id);
  };
  const log=[],results=[];
  let picked="content://downloads/document/neo",saved="content://downloads/document/backup";
  const api={
    core:{async invoke(cmd,args){
      log.push({cmd,args});
      if(cmd==="hybrid_checkpoint_export_begin")return {bytes:4,step:123};
      if(cmd==="hybrid_checkpoint_export_android")return {bytes:4,verified:true};
      if(cmd==="hybrid_checkpoint_export_chunk")return [1,2,3,4];
      if(cmd==="hybrid_status")return {training:false};
      if(cmd==="hybrid_checkpoint_import_finish")return {transformerStep:123,tokenizerWords:4};
      return null;
    }},
    dialog:{
      async save(){log.push({cmd:"dialog.save"});return saved;},
      async open(){log.push({cmd:"dialog.open"});return picked;},
    },
    fs:{async readFile(uri){log.push({cmd:"fs.readFile",uri});return new Uint8Array([1,2,3,4]);}}
  };
  const document={
    getElementById:dom,createElement:()=>({click(){},remove(){}}),
    body:{append(){}}
  };
  const context={
    window:{__TAURI__:api},document,navigator:{userAgent},
    Blob,Uint8Array,Number,String,Math,Array,Promise,Error,console:{log(){},error(){}},
    setTimeout(callback){callback();},confirm:()=>true,
    URL:{createObjectURL(){return "blob:test";},revokeObjectURL(){}},
  };
  vm.runInNewContext(script,context,{filename:"checkpoint-ui.js"});
  return {dom,log,api,setSaved:s=>{saved=s;},setPicked:s=>{picked=s;}};
}

async function main(){
  {
    const t=mock("Android");
    assert.equal(t.dom("importCheckpointNative").hidden,false);
    assert.equal(t.dom("importCheckpointFile").hidden,true);
    await t.dom("exportCheckpoint").click();
    assert.equal(t.log.some(x=>x.cmd==="dialog.save"),true);
    assert.equal(t.log.some(x=>x.cmd==="hybrid_checkpoint_export_android" &&
      x.args.destinationUri==="content://downloads/document/backup"),true);
    assert.equal(t.log.some(x=>x.cmd==="hybrid_checkpoint_export_chunk"),false,
      "Android must never download a giant Blob through WebView");
    assert.equal(t.log.some(x=>x.cmd==="hybrid_checkpoint_export_cancel"),true);
    assert.match(t.dom("checkpointTransferStatus").textContent,/SHA-256 verified/);
  }
  {
    const t=mock("Android");
    t.setSaved(null);
    await t.dom("exportCheckpoint").click();
    assert.match(t.dom("checkpointTransferStatus").textContent,/cancelled/);
    assert.equal(t.log.some(x=>x.cmd==="hybrid_checkpoint_export_android"),false);
    assert.equal(t.log.some(x=>x.cmd==="hybrid_checkpoint_export_cancel"),true);
  }
  {
    const t=mock("Android");
    await t.dom("importCheckpointNative").click();
    assert.equal(t.dom("importCheckpoint").disabled,false);
    await t.dom("importCheckpoint").click();
    assert.equal(t.log.some(x=>x.cmd==="dialog.open"),true);
    assert.equal(t.log.some(x=>x.cmd==="fs.readFile"),true);
    assert.equal(t.log.some(x=>x.cmd==="hybrid_checkpoint_import_append"),true);
    assert.equal(t.log.some(x=>x.cmd==="hybrid_checkpoint_import_finish"),true);
    assert.match(t.dom("checkpointTransferStatus").textContent,/Restored whole joint brain/);
  }
  {
    const t=mock("Windows NT 10.0");
    await t.dom("exportCheckpoint").click();
    assert.equal(t.log.some(x=>x.cmd==="hybrid_checkpoint_export_chunk"),true);
    assert.equal(t.log.some(x=>x.cmd==="dialog.save"),false);
    assert.equal(t.log.some(x=>x.cmd==="hybrid_checkpoint_export_cancel"),true);
  }
  console.log("checkpoint-ui: Android native save, cancel, native restore, and Windows export passed");
}
main().catch(e=>{console.error(e);process.exitCode=1;});
