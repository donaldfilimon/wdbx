import { resolve } from 'node:path';
export const config = {
 runner:'local',specs:['./tests/native.e2e.mjs'],maxInstances:1,logLevel:'warn',framework:'mocha',reporters:['spec'],mochaOpts:{timeout:120000},waitforTimeout:30000,connectionRetryTimeout:120000,
 services:[['@wdio/tauri-service',{appBinaryPath:resolve('target/debug/wdbx-studio-desktop'+(process.platform==='win32'?'.exe':'')),driverProvider:'embedded',captureBackendLogs:true,captureFrontendLogs:true}]],
 capabilities:[{browserName:'tauri'}]
};
