// Run: node scripts/stdio-smoke.cjs <server.exe>
const fs = require('node:fs');
const path = require('node:path');
const {pathToFileURL} = require('node:url');
const {spawn} = require('node:child_process');
const assert = require('node:assert/strict');
const base = path.resolve('.local/stdio smoke');
fs.mkdirSync(base, {recursive:true});
const exe = path.join(base, 'server with spaces.exe');
fs.copyFileSync(path.resolve(process.argv[2]), exe);
const roots = ['root one','root two'].map(n=>path.join(base,n));
roots.forEach(p=>fs.mkdirSync(p,{recursive:true}));
const uri = p=>pathToFileURL(p).href;
const trace = [];
const child = spawn(exe, [], {stdio:['pipe','pipe','pipe']});
let buffer=Buffer.alloc(0), serial=0;
const pending=new Map();
let stderr=''; child.stderr.on('data', b=>stderr+=b);
function send(message) {
  trace.push({direction:'client',message});
  const body=Buffer.from(JSON.stringify({jsonrpc:'2.0',...message}));
  child.stdin.write(`Content-Length: ${body.length}\r\n\r\n`); child.stdin.write(body);
}
function request(method,params) {
  const id=++serial;
  return new Promise((resolve,reject)=>{
    const timer=setTimeout(()=>{pending.delete(id); reject(Error(`Timeout: ${method}`));},15000);
    pending.set(id,m=>{clearTimeout(timer); m.error?reject(Error(JSON.stringify(m.error))):resolve(m.result);});
    send({id,method,params});
  });
}
child.stdout.on('data', chunk=>{
  buffer=Buffer.concat([buffer,chunk]);
  while(true) {
    const end=buffer.indexOf('\r\n\r\n'); if(end<0)return;
    const match=/^Content-Length: (\d+)$/im.exec(buffer.subarray(0,end).toString());
    assert(match,'stdout must contain only framed JSON-RPC');
    const size=Number(match[1]); if(buffer.length<end+4+size)return;
    const message=JSON.parse(buffer.subarray(end+4,end+4+size)); buffer=buffer.subarray(end+4+size);
    trace.push({direction:'server',message});
    if(message.method && message.id!==undefined) {
      send({id:message.id,result:message.method==='workspace/workspaceFolders'?roots.map(p=>({uri:uri(p),name:path.basename(p)})):null});
    } else if(pending.has(message.id)) {pending.get(message.id)(message);pending.delete(message.id);}
  }
});
(async()=>{
  const capabilities=await request('initialize',{processId:process.pid,rootUri:uri(roots[0]),workspaceFolders:roots.map(p=>({uri:uri(p),name:path.basename(p)})),capabilities:{workspace:{workspaceFolders:true,didChangeWatchedFiles:{dynamicRegistration:true}},textDocument:{hover:{contentFormat:['markdown']},completion:{completionItem:{snippetSupport:true}}}},initializationOptions:{lens_enabled:false,ini_path:''}});
  assert(!capabilities.capabilities.codeLensProvider);
  assert.equal(capabilities.capabilities.documentRangeFormattingProvider,true);
  send({method:'initialized',params:{}});
  await new Promise(r=>setTimeout(r,1500));
  const file=path.join(roots[0],'sample.prg');
  const text='func привет() { return 1; }\nvar число=привет();\n';
  fs.writeFileSync(file,text);
  const doc={uri:uri(file)};
  send({method:'textDocument/didOpen',params:{textDocument:{...doc,languageId:'stack',version:1,text}}});
  await new Promise(r=>setTimeout(r,400));
  const results={capabilities};
  for(const method of ['hover','completion','documentSymbol','semanticTokens/full','rangeFormatting']) {
    const params={textDocument:doc,position:{line:1,character:14},range:{start:{line:0,character:0},end:{line:2,character:0}},options:{tabSize:4,insertSpaces:true},context:{triggerKind:1}};
    results[method]=await request('textDocument/'+method,params);
  }
  send({method:'textDocument/didChange',params:{textDocument:{...doc,version:2},contentChanges:[{text:'func broken( {\n'}]}});
  await new Promise(r=>setTimeout(r,400));
  send({method:'textDocument/didChange',params:{textDocument:{...doc,version:3},contentChanges:[{text}]}});
  await new Promise(r=>setTimeout(r,400));
  await request('shutdown'); send({method:'exit'});
  results.actualExecutable=exe;
  results.diagnostics=trace.filter(e=>e.message.method==='textDocument/publishDiagnostics');
  results.serverRequests=trace.filter(e=>e.direction==='server'&&e.message.method&&e.message.id!==undefined);
  assert(results.hover, 'Expected hover content');
  assert(results.completion, 'Expected completion response');
  assert(results.documentSymbol?.length, 'Expected document symbols');
  assert(results.rangeFormatting?.length, 'Expected formatting edits');
  assert(results['semanticTokens/full']?.data?.length, 'Expected semantic tokens');
  assert(results.diagnostics.some(e=>e.message.params.version===2 && e.message.params.diagnostics.length), 'Expected diagnostics for broken text');
  assert(results.diagnostics.some(e=>e.message.params.version===3 && !e.message.params.diagnostics.length), 'Expected cleared diagnostics after repair');
  fs.writeFileSync(path.join(base,'results.json'),JSON.stringify(results,null,2));
  console.log(JSON.stringify({executable:exe,hover:!!results.hover,completion:!!results.completion,symbols:results.documentSymbol?.length,formatEdits:results.rangeFormatting?.length,diagnostics:results.diagnostics.length,serverRequests:results.serverRequests.map(e=>e.message.method)}));
})().catch(e=>{console.error(e);process.exitCode=1;}).finally(()=>{
  fs.writeFileSync(path.join(base,'trace.json'),JSON.stringify(trace,null,2));
  fs.writeFileSync(path.join(base,'stderr.txt'),stderr);
  child.kill();
});
