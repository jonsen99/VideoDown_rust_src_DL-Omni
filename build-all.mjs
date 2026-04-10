import { execSync } from 'child_process';
import fs from 'fs';
import path from 'path';

// 1. 读取 tauri.conf.json 获取当前版本号
const tauriConfPath = path.join('src-tauri', 'tauri.conf.json');
const tauriConf = JSON.parse(fs.readFileSync(tauriConfPath, 'utf-8'));
const version = tauriConf.version;

// 2. 目录设置
const bundleDir = path.join('src-tauri', 'target', 'release', 'bundle');
const outDir = path.join('release-builds');

// 如果输出目录不存在，则创建
if (!fs.existsSync(outDir)) {
    fs.mkdirSync(outDir);
}

// 辅助函数：移动并重命名安装包
function collectArtifacts(suffix = '') {
    const msiName = `dl-omni_${version}_x64_en-US.msi`;
    const nsisName = `dl-omni_${version}_x64-setup.exe`;

    const msiSrc = path.join(bundleDir, 'msi', msiName);
    const nsisSrc = path.join(bundleDir, 'nsis', nsisName);

    const msiDest = path.join(outDir, `dl-omni_${version}_x64_en-US${suffix}.msi`);
    const nsisDest = path.join(outDir, `dl-omni_${version}_x64-setup${suffix}.exe`);

    if (fs.existsSync(msiSrc)) {
        fs.renameSync(msiSrc, msiDest);
        console.log(`✅ 已生成: ${msiDest}`);
    } else {
        console.warn(`⚠️ 未找到预期的文件: ${msiSrc}`);
    }

    if (fs.existsSync(nsisSrc)) {
        fs.renameSync(nsisSrc, nsisDest);
        console.log(`✅ 已生成: ${nsisDest}`);
    } else {
        console.warn(`⚠️ 未找到预期的文件: ${nsisSrc}`);
    }
}

try {
    console.log("🚀 ======================================");
    console.log(`🚀 开始构建 Lite 版 (v${version})...`);
    console.log("🚀 ======================================");
    // 执行普通打包 (不带引擎资源)
    execSync('npm run tauri build', { stdio: 'inherit' });
    collectArtifacts(''); // 提取 Lite 版文件

    console.log("\n🚀 ======================================");
    console.log(`🚀 开始构建 Full 版 (v${version})...`);
    console.log("🚀 ======================================");
    // 修正了这里的路径，加上了 src-tauri/
    execSync('npm run tauri build -- --config src-tauri/tauri.full.json', { stdio: 'inherit' });
    collectArtifacts('-full'); // 提取 Full 版文件，添加 -full 后缀

    console.log("\n🎉 打包完成！所有 4 个安装包已整理至项目的 [release-builds] 文件夹中。");
} catch (error) {
    console.error("\n❌ 打包过程中发生错误:", error.message);
}