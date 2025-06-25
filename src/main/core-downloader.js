/**
 * 内核下载器模块
 * 负责下载和安装sing-box内核
 */
const { app } = require('electron');
const path = require('path');
const fs = require('fs');
const os = require('os');
const https = require('https');
const { pipeline } = require('stream');
const { promisify } = require('util');
const streamPipeline = promisify(pipeline);
const logger = require('../utils/logger');
const { getAppDataDir, getBinDir } = require('../utils/paths');

let AdmZip;
try {
  AdmZip = require('adm-zip');
} catch (error) {
  logger.warn('AdmZip库未安装，解压功能将不可用');
}

/**
 * 检查URL是否可达
 * @param {string} url 要检查的URL
 * @param {number} timeout 超时时间（毫秒）
 * @returns {Promise<boolean>} 是否可达
 */
function checkUrlAvailability(url, timeout = 5000) {
  return new Promise((resolve) => {
    const urlObj = new URL(url);
    const options = {
      hostname: urlObj.hostname,
      path: urlObj.pathname + urlObj.search,
      method: 'HEAD',
      timeout: timeout,
      headers: {
        'User-Agent': 'lvory-downloader'
      }
    };

    const request = https.request(options, (response) => {
      resolve(response.statusCode >= 200 && response.statusCode < 400);
    });

    request.on('error', () => {
      resolve(false);
    });

    request.on('timeout', () => {
      request.destroy();
      resolve(false);
    });

    request.end();
  });
}

/**
 * 下载内核函数
 * @param {Object} options 下载选项
 * @returns {Promise<Object>} 下载结果
 */
const downloadCore = async (mainWindow) => {
  try {
    // 检查是否有AdmZip库
    if (!AdmZip) {
      throw new Error('解压库未安装，无法下载和解压内核');
    }
    
    const binDir = getBinDir();
    
    // 获取当前平台信息
    const platform = process.platform;
    const arch = process.arch === 'x64' ? 'amd64' : 
                 process.arch === 'arm64' ? 'arm64' : 'amd64';
    
    const version = '1.11.9';
    let downloadUrl;
    let githubDownloadUrl;
    let binaryName;
    let archiveName;
    let extractedFolderName;
    
    if (platform === 'win32') {
      downloadUrl = `https://oss.sxueck.com/singbox/sing-box-${version}-windows-${arch}.zip`;
      githubDownloadUrl = `https://github.com/SagerNet/sing-box/releases/download/v${version}/sing-box-${version}-windows-${arch}.zip`;
      binaryName = 'sing-box.exe';
      archiveName = 'sing-box.zip';
      extractedFolderName = `sing-box-${version}-windows-${arch}`;
    } else if (platform === 'darwin') {
      downloadUrl = `https://oss.sxueck.com/singbox/sing-box-${version}-darwin-${arch}.tar.gz`;
      githubDownloadUrl = `https://github.com/SagerNet/sing-box/releases/download/v${version}/sing-box-${version}-darwin-${arch}.tar.gz`;
      binaryName = 'sing-box';
      archiveName = 'sing-box.tar.gz';
      extractedFolderName = `sing-box-${version}-darwin-${arch}`;
    } else if (platform === 'linux') {
      downloadUrl = `https://oss.sxueck.com/singbox/sing-box-${version}-linux-${arch}.tar.gz`;
      githubDownloadUrl = `https://github.com/SagerNet/sing-box/releases/download/v${version}/sing-box-${version}-linux-${arch}.tar.gz`;
      binaryName = 'sing-box';
      archiveName = 'sing-box.tar.gz';
      extractedFolderName = `sing-box-${version}-linux-${arch}`;
    } else {
      throw new Error(`不支持的平台: ${platform}`);
    }
    
    const targetPath = path.join(binDir, binaryName);
    const appDataDir = getAppDataDir();
    const tempFilePath = path.join(appDataDir, 'temp_download');
    
    // 确保临时文件夹存在
    if (!fs.existsSync(tempFilePath)) {
      fs.mkdirSync(tempFilePath, { recursive: true });
    }
    
    // 下载文件的保存路径
    const archivePath = path.join(tempFilePath, archiveName);
    
    logger.info(`下载sing-box: ${downloadUrl} 到 ${archivePath}`);
    
    if (mainWindow && !mainWindow.isDestroyed()) {
      mainWindow.webContents.send('core-download-progress', {
        progress: 0,
        message: `开始下载: ${downloadUrl}`
      });
    }
    
    // 如果存在旧的压缩文件，删除它
    if (fs.existsSync(archivePath)) {
      fs.unlinkSync(archivePath);
    }
    
    // 创建可写流
    let fileStream = fs.createWriteStream(archivePath);
    
    // 使用https下载文件 - 创建一个支持重定向的下载函数
    const downloadWithRedirects = async (url, maxRedirects = 5) => {
      let redirectCount = 0;
      let currentUrl = url;
      
      while (redirectCount < maxRedirects) {
        const urlObj = new URL(currentUrl);
        const options = {
          hostname: urlObj.hostname,
          path: urlObj.pathname + urlObj.search,
          timeout: 30000, // 30秒超时
          headers: {
            'User-Agent': 'lvory-Downloader'
          }
        };
        
        try {
          await new Promise((resolve, reject) => {
            const request = https.get(options, async (response) => {
              // 处理重定向
              if (response.statusCode >= 300 && response.statusCode < 400 && response.headers.location) {
                redirectCount++;
                currentUrl = response.headers.location;
                fileStream.close();
                resolve();
                return;
              }
              
              if (response.statusCode !== 200) {
                reject(new Error(`状态码: ${response.statusCode}`));
                return;
              }
              
              const totalBytes = parseInt(response.headers['content-length'] || '0', 10);
              let downloadedBytes = 0;
              
              response.on('data', (chunk) => {
                downloadedBytes += chunk.length;
                if (totalBytes > 0 && mainWindow && !mainWindow.isDestroyed()) {
                  const progress = Math.floor((downloadedBytes / totalBytes) * 100);
                  mainWindow.webContents.send('core-download-progress', {
                    progress,
                    message: `已下载: ${progress}%`
                  });
                }
              });
              
              try {
                await streamPipeline(response, fileStream);
                resolve();
              } catch (err) {
                reject(err);
              }
            });
            
            request.on('error', (err) => {
              reject(err);
            });

            request.on('timeout', () => {
              request.destroy();
              reject(new Error('下载超时'));
            });
          });
          
          // 如果没有重定向，跳出循环
          break;
        } catch (error) {
          if (redirectCount >= maxRedirects) {
            throw new Error(`超过最大重定向次数: ${maxRedirects}`);
          }
          // 继续下一次重定向尝试
        }
      }
    };
    
    try {
      if (platform === 'win32' || platform === 'linux' || platform === 'darwin') {
        logger.info(`检查OSS连通性: ${downloadUrl}`);
        
        if (mainWindow && !mainWindow.isDestroyed()) {
          mainWindow.webContents.send('core-download-progress', {
            progress: 0,
            message: `正在检查OSS连通性...`
          });
        }

        const ossAvailable = await checkUrlAvailability(downloadUrl, 3000);
        
        if (ossAvailable) {
          logger.info(`OSS连通性检查通过，开始从OSS下载: ${downloadUrl}`);
          
          if (mainWindow && !mainWindow.isDestroyed()) {
            mainWindow.webContents.send('core-download-progress', {
              progress: 0,
              message: `正在从 OSS 源下载内核`
            });
          }
          
          try {
            await downloadWithRedirects(downloadUrl);
            logger.info(`从OSS ${downloadUrl} 下载成功`);
          } catch (error) {
            logger.warn(`从OSS下载失败: ${error.message}，尝试从GitHub下载`);
            throw error; // 抛出错误以触发后备下载
          }
        } else {
          logger.warn(`OSS连通性检查失败，直接尝试从GitHub下载`);
          throw new Error('OSS不可达');
        }
      } else {
        // 其他平台直接从GitHub下载
        if (mainWindow && !mainWindow.isDestroyed()) {
          mainWindow.webContents.send('core-download-progress', {
            progress: 0,
            message: `开始下载: ${downloadUrl}`
          });
        }
        
        await downloadWithRedirects(downloadUrl);
        logger.info(`从 ${downloadUrl} 下载成功`);
      }
    } catch (error) {
      // 如果有GitHub备用地址，尝试从GitHub下载
      if ((platform === 'win32' || platform === 'linux' || platform === 'darwin') && githubDownloadUrl) {
        logger.info(`尝试从GitHub下载: ${githubDownloadUrl}`);
        
        if (mainWindow && !mainWindow.isDestroyed()) {
          mainWindow.webContents.send('core-download-progress', {
            progress: 0,
            message: `从GitHub下载中`
          });
        }
        
        // 重新创建文件流
        if (fs.existsSync(archivePath)) {
          fs.unlinkSync(archivePath);
        }
        
        const newFileStream = fs.createWriteStream(archivePath);
        fileStream.close();
        fileStream = newFileStream;
        
        try {
          await downloadWithRedirects(githubDownloadUrl);
          logger.info(`从GitHub ${githubDownloadUrl} 下载成功`);
        } catch (githubError) {
          throw new Error(`所有下载源都失败了。OSS错误: ${error.message}，GitHub错误: ${githubError.message}`);
        }
      } else {
        throw new Error(`下载文件失败: ${error.message}`);
      }
    }
    
    logger.info(`下载成功，准备解压`);
    
    if (mainWindow && !mainWindow.isDestroyed()) {
      mainWindow.webContents.send('core-download-progress', {
        progress: 90,
        message: `正在解压和安装...`
      });
    }
    
    const extractDir = path.join(tempFilePath, 'extracted');
    if (fs.existsSync(extractDir)) {
      fs.rmSync(extractDir, { recursive: true, force: true });
    }
    fs.mkdirSync(extractDir, { recursive: true });
    
    // 根据平台选择解压方式
    if (platform === 'win32') {
      // Windows使用AdmZip直接解压
      const zip = new AdmZip(archivePath);
      zip.extractAllTo(extractDir, true);
      
      // 复制到bin目录
      const exePath = path.join(extractDir, extractedFolderName, binaryName);
      if (!fs.existsSync(exePath)) {
        throw new Error(`未找到${binaryName}文件`);
      }
      
      fs.copyFileSync(exePath, targetPath);
    } else {
      // macOS/Linux使用命令行解压tar.gz
      const { exec } = require('child_process');
      await new Promise((resolve, reject) => {
        exec(`tar -xzf "${archivePath}" -C "${extractDir}"`, (error) => {
          if (error) {
            reject(error);
            return;
          }
          resolve();
        });
      });
      
      // 复制到bin目录
      const exePath = path.join(extractDir, extractedFolderName, binaryName);
      if (!fs.existsSync(exePath)) {
        throw new Error(`未找到${binaryName}文件`);
      }
      
      fs.copyFileSync(exePath, targetPath);
      
      // 设置执行权限
      await new Promise((resolve, reject) => {
        exec(`chmod +x "${targetPath}"`, (error) => {
          if (error) {
            reject(error);
            return;
          }
          resolve();
        });
      });
    }
    
    logger.info(`核心已安装到: ${targetPath}`);
    
    // 清理临时文件
    try {
      if (fs.existsSync(archivePath)) {
        fs.unlinkSync(archivePath);
      }
      if (fs.existsSync(extractDir)) {
        fs.rmSync(extractDir, { recursive: true, force: true });
      }
    } catch (err) {
      logger.error('清理临时文件失败:', err);
    }
    
    if (mainWindow && !mainWindow.isDestroyed()) {
      mainWindow.webContents.send('core-download-progress', {
        progress: 100,
        message: `安装完成!`
      });
    }
    
    return { 
      success: true, 
      version: `v${version}`,
      path: targetPath
    };
  } catch (error) {
    logger.error(`下载sing-box失败: ${error.message}`);
    if (mainWindow && !mainWindow.isDestroyed()) {
      mainWindow.webContents.send('core-download-progress', {
        progress: -1,
        message: `下载失败: ${error.message}`
      });
    }
    return { 
      success: false, 
      error: error.message || '下载失败，请检查网络连接'
    };
  }
};

module.exports = {
  downloadCore
}; 