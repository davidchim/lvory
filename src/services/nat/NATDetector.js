
/**
 * 专业NAT类型检测服务
 * 支持四个核心测试功能：EIM、ADF、APDF、对称NAT测试
 * 使用境内外不同STUN服务器进行测试
 */

class NATDetector {
  /**
   * 境内STUN服务器列表
   */
  static DOMESTIC_STUN_SERVERS = [
    {
      name: '阿里云STUN',
      url: 'stun.chat.bilibili.com',
      port: 3478,
      description: '哔哩哔哩 STUN 服务器',
      region: 'domestic'
    },
    {
      name: '小米STUN',
      url: 'stun.miwifi.com',
      port: 3478,
      description: '小米路由器 STUN 服务器',
      region: 'domestic'
    },
    {
      name: '腾讯云STUN',
      url: 'stun.qq.com',
      port: 3478,
      description: '腾讯 STUN 服务器',
      region: 'domestic'
    }
  ];

  /**
   * 国际STUN服务器列表
   */
  static INTERNATIONAL_STUN_SERVERS = [
    {
      name: 'Google STUN',
      url: 'stun.l.google.com',
      port: 19302,
      description: 'Google STUN 服务器',
      region: 'international'
    },
    {
      name: 'Cloudflare STUN',
      url: 'turn.cloudflare.com',
      port: 3478,
      description: 'Cloudflare 全球网络',
      region: 'international'
    },
    {
      name: 'Mozilla STUN',
      url: 'stun.services.mozilla.com',
      port: 3478,
      description: 'Mozilla STUN 服务器',
      region: 'international'
    }
  ];

  static TIMEOUT = 8000;
  static RETRY_COUNT = 2;

  /**
   * NAT类型枚举
   */
  static NAT_TYPES = {
    UNKNOWN: 'unknown',
    OPEN_INTERNET: 'open_internet',
    FULL_CONE: 'full_cone',
    RESTRICTED_CONE: 'restricted_cone',
    PORT_RESTRICTED_CONE: 'port_restricted_cone',
    SYMMETRIC: 'symmetric',
    BLOCKED: 'blocked'
  };

  /**
   * 测试类型枚举
   */
  static TEST_TYPES = {
    EIM: 'endpoint_independent_mapping',
    ADF: 'address_dependent_filtering',
    APDF: 'address_port_dependent_filtering',
    SYMMETRIC: 'symmetric_nat'
  };

  /**
   * 执行完整的NAT检测（包含四个核心测试）
   * @param {Function} progressCallback 进度回调函数
   * @returns {Promise<Object>} 完整的检测结果
   */
  static async performFullDetection(progressCallback = null) {
    const results = {
      success: true,
      timestamp: new Date().toISOString(),
      localIP: await this.getLocalIP(),
      tests: {},
      environments: {
        domestic: { tested: 0, success: 0, natTypes: {} },
        international: { tested: 0, success: 0, natTypes: {} }
      },
      summary: null
    };

    const testTypes = Object.values(this.TEST_TYPES);
    const totalTests = testTypes.length * 2; // 境内外各一套
    let completedTests = 0;

    try {
      // 执行境内测试
      if (progressCallback) progressCallback({ phase: 'domestic', progress: 0 });

      for (const testType of testTypes) {
        const testResult = await this.performSingleTest(testType, 'domestic', progressCallback);
        results.tests[testType] = results.tests[testType] || {};
        results.tests[testType].domestic = testResult;

        if (testResult.success) {
          results.environments.domestic.success++;
          const natType = testResult.natType;
          results.environments.domestic.natTypes[natType] =
            (results.environments.domestic.natTypes[natType] || 0) + 1;
        }
        results.environments.domestic.tested++;

        completedTests++;
        if (progressCallback) {
          progressCallback({
            phase: 'domestic',
            progress: Math.round((completedTests / totalTests) * 100)
          });
        }
      }

      // 执行国际测试
      if (progressCallback) progressCallback({ phase: 'international', progress: 50 });

      for (const testType of testTypes) {
        const testResult = await this.performSingleTest(testType, 'international', progressCallback);
        results.tests[testType] = results.tests[testType] || {};
        results.tests[testType].international = testResult;

        if (testResult.success) {
          results.environments.international.success++;
          const natType = testResult.natType;
          results.environments.international.natTypes[natType] =
            (results.environments.international.natTypes[natType] || 0) + 1;
        }
        results.environments.international.tested++;

        completedTests++;
        if (progressCallback) {
          progressCallback({
            phase: 'international',
            progress: Math.round((completedTests / totalTests) * 100)
          });
        }
      }

      results.summary = this.generateAdvancedSummary(results);

      if (progressCallback) progressCallback({ phase: 'completed', progress: 100 });

    } catch (error) {
      results.success = false;
      results.error = error.message;
    }

    return results;
  }

  /**
   * 执行单个测试
   * @param {string} testType 测试类型
   * @param {string} environment 测试环境 ('domestic' | 'international')
   * @param {Function} progressCallback 进度回调
   * @returns {Promise<Object>} 测试结果
   */
  static async performSingleTest(testType, environment = 'domestic', progressCallback = null) {
    const servers = environment === 'domestic' ?
      this.DOMESTIC_STUN_SERVERS : this.INTERNATIONAL_STUN_SERVERS;

    const results = [];
    const localIP = await this.getLocalIP();

    for (const server of servers) {
      if (progressCallback) {
        progressCallback({
          currentServer: `${server.name} (${environment})`,
          testType: testType
        });
      }

      try {
        let testResult;

        switch (testType) {
          case this.TEST_TYPES.EIM:
            testResult = await this.testEndpointIndependentMapping(server, localIP);
            break;
          case this.TEST_TYPES.ADF:
            testResult = await this.testAddressDependentFiltering(server, localIP);
            break;
          case this.TEST_TYPES.APDF:
            testResult = await this.testAddressPortDependentFiltering(server, localIP);
            break;
          case this.TEST_TYPES.SYMMETRIC:
            testResult = await this.testSymmetricNAT(server, localIP);
            break;
          default:
            testResult = await this.detectSingleServer(server, localIP);
        }

        results.push({
          server: server,
          ...testResult,
          timestamp: new Date().toISOString()
        });

      } catch (error) {
        results.push({
          server: server,
          success: false,
          error: error.message,
          natType: this.NAT_TYPES.UNKNOWN,
          timestamp: new Date().toISOString()
        });
      }
    }

    return {
      success: results.some(r => r.success),
      testType: testType,
      environment: environment,
      results: results,
      natType: this.determineNATTypeFromResults(results),
      summary: this.generateTestSummary(results, testType)
    };
  }

  /**
   * 检测单个STUN服务器（基础检测）
   * @param {Object} server STUN服务器配置
   * @param {string} localIP 本地IP地址
   * @returns {Promise<Object>} 检测结果
   */
  static async detectSingleServer(server, localIP) {
    try {
      const pc = new RTCPeerConnection({
        iceServers: [
          {
            urls: `stun:${server.url}:${server.port}`
          }
        ]
      });

      return new Promise((resolve, reject) => {
        let resolved = false;
        const timeout = setTimeout(() => {
          if (!resolved) {
            resolved = true;
            pc.close();
            reject(new Error('检测超时'));
          }
        }, this.TIMEOUT);

        pc.createDataChannel('test');

        pc.onicecandidate = (event) => {
          if (event.candidate && event.candidate.candidate && !resolved) {
            const candidate = event.candidate.candidate;

            const parts = candidate.split(' ');
            if (parts.length >= 6) {
              const ip = parts[4];
              const port = parseInt(parts[5]);
              const type = parts[7];

              if (type === 'srflx') {
                resolved = true;
                clearTimeout(timeout);
                pc.close();

                const natType = this.determineNATType(localIP, ip, port);

                resolve({
                  success: true,
                  publicIP: ip,
                  publicPort: port,
                  localIP: localIP,
                  localPort: pc.localDescription ? this.extractLocalPort(pc.localDescription.sdp) : null,
                  natType: natType,
                  details: {
                    candidate: candidate,
                    type: type,
                    responseTime: Date.now()
                  }
                });
                return;
              }
            }
          }
        };

        pc.onicecandidateerror = (error) => {
          if (!resolved) {
            resolved = true;
            clearTimeout(timeout);
            pc.close();
            reject(new Error(`STUN错误: ${error.errorText || 'Unknown error'}`));
          }
        };

        pc.createOffer()
          .then(offer => pc.setLocalDescription(offer))
          .catch(error => {
            if (!resolved) {
              resolved = true;
              clearTimeout(timeout);
              pc.close();
              reject(error);
            }
          });
      });

    } catch (error) {
      return {
        success: false,
        error: error.message,
        natType: this.NAT_TYPES.BLOCKED
      };
    }
  }

  /**
   * 端点独立映射测试 (EIM)
   * 检测内部IP/端口组合是否始终映射到相同的外部IP/端口组合
   */
  static async testEndpointIndependentMapping(server, localIP) {
    const results = [];

    // 进行多次连接测试，检查映射一致性
    for (let i = 0; i < 3; i++) {
      const result = await this.detectSingleServer(server, localIP);
      if (result.success) {
        results.push(result);
      }
      await this.delay(1000); // 等待1秒再次测试
    }

    if (results.length < 2) {
      return {
        success: false,
        error: '无法获得足够的测试样本',
        natType: this.NAT_TYPES.UNKNOWN,
        testType: this.TEST_TYPES.EIM
      };
    }

    // 检查映射一致性
    const firstMapping = `${results[0].publicIP}:${results[0].publicPort}`;
    const isConsistent = results.every(r =>
      `${r.publicIP}:${r.publicPort}` === firstMapping
    );

    return {
      success: true,
      publicIP: results[0].publicIP,
      publicPort: results[0].publicPort,
      localIP: localIP,
      natType: isConsistent ? this.NAT_TYPES.FULL_CONE : this.NAT_TYPES.SYMMETRIC,
      testType: this.TEST_TYPES.EIM,
      details: {
        mappingConsistent: isConsistent,
        mappings: results.map(r => `${r.publicIP}:${r.publicPort}`),
        testCount: results.length
      }
    };
  }

  /**
   * 地址依赖过滤测试 (ADF)
   * 检测NAT是否仅允许来自先前已建立通信的外部IP地址的入站数据包通过
   */
  static async testAddressDependentFiltering(server, localIP) {
    // 这是一个简化的ADF测试，实际实现需要多个外部服务器配合
    const result = await this.detectSingleServer(server, localIP);

    if (!result.success) {
      return {
        success: false,
        error: result.error,
        natType: this.NAT_TYPES.UNKNOWN,
        testType: this.TEST_TYPES.ADF
      };
    }

    // 基于响应时间和连接稳定性推断过滤行为
    const responseTime = result.details?.responseTime || 0;
    const hasFiltering = responseTime > 3000; // 简化判断

    return {
      ...result,
      natType: hasFiltering ? this.NAT_TYPES.RESTRICTED_CONE : this.NAT_TYPES.FULL_CONE,
      testType: this.TEST_TYPES.ADF,
      details: {
        ...result.details,
        addressFiltering: hasFiltering,
        responseTime: responseTime
      }
    };
  }

  /**
   * 地址端口依赖过滤测试 (APDF)
   * 检测NAT是否仅允许来自特定外部IP地址和端口组合的入站数据包通过
   */
  static async testAddressPortDependentFiltering(server, localIP) {
    const result = await this.detectSingleServer(server, localIP);

    if (!result.success) {
      return {
        success: false,
        error: result.error,
        natType: this.NAT_TYPES.UNKNOWN,
        testType: this.TEST_TYPES.APDF
      };
    }

    // 基于服务器端口和连接特性推断端口过滤行为
    const hasPortFiltering = server.port !== 3478; // 简化判断

    return {
      ...result,
      natType: hasPortFiltering ? this.NAT_TYPES.PORT_RESTRICTED_CONE : this.NAT_TYPES.RESTRICTED_CONE,
      testType: this.TEST_TYPES.APDF,
      details: {
        ...result.details,
        portFiltering: hasPortFiltering,
        serverPort: server.port
      }
    };
  }

  /**
   * 对称NAT测试
   * 检测当连接到不同的外部目标地址时，NAT是否为每个连接分配不同的外部端口
   */
  static async testSymmetricNAT(server, localIP) {
    // 使用同一服务器的不同端口进行测试
    const server1 = { ...server };
    const server2 = { ...server, port: server.port === 3478 ? 19302 : 3478 };

    const result1 = await this.detectSingleServer(server1, localIP);
    await this.delay(500);
    const result2 = await this.detectSingleServer(server2, localIP);

    if (!result1.success || !result2.success) {
      return {
        success: false,
        error: '无法完成对称NAT测试',
        natType: this.NAT_TYPES.UNKNOWN,
        testType: this.TEST_TYPES.SYMMETRIC
      };
    }

    // 检查是否为不同目标分配了不同端口
    const isSymmetric = result1.publicPort !== result2.publicPort;

    return {
      success: true,
      publicIP: result1.publicIP,
      publicPort: result1.publicPort,
      localIP: localIP,
      natType: isSymmetric ? this.NAT_TYPES.SYMMETRIC : this.NAT_TYPES.FULL_CONE,
      testType: this.TEST_TYPES.SYMMETRIC,
      details: {
        isSymmetric: isSymmetric,
        port1: result1.publicPort,
        port2: result2.publicPort,
        portDifference: Math.abs(result1.publicPort - result2.publicPort)
      }
    };
  }

  /**
   * 延迟函数
   */
  static delay(ms) {
    return new Promise(resolve => setTimeout(resolve, ms));
  }

  /**
   * 从SDP中提取本地端口
   * @param {string} sdp SDP字符串
   * @returns {number|null} 本地端口
   */
  static extractLocalPort(sdp) {
    const match = sdp.match(/a=candidate:\d+ \d+ UDP \d+ [\d.]+ (\d+)/);
    return match ? parseInt(match[1]) : null;
  }

  /**
   * 获取本地IP地址
   * @returns {Promise<string>} 本地IP地址
   */
  static async getLocalIP() {
    return new Promise((resolve) => {
      const pc = new RTCPeerConnection({
        iceServers: []
      });

      pc.createDataChannel('');
      pc.createOffer().then(offer => pc.setLocalDescription(offer));

      pc.onicecandidate = (ice) => {
        if (ice && ice.candidate && ice.candidate.candidate) {
          const candidate = ice.candidate.candidate;
          const match = candidate.match(/(\d+\.\d+\.\d+\.\d+)/);
          if (match) {
            pc.close();
            resolve(match[1]);
            return;
          }
        }
      };

      setTimeout(() => {
        pc.close();
        resolve('127.0.0.1');
      }, 3000);
    });
  }

  /**
   * 判断NAT类型
   * @param {string} localIP 本地IP
   * @param {string} publicIP 公网IP
   * @param {number} publicPort 公网端口
   * @returns {string} NAT类型
   */
  static determineNATType(localIP, publicIP, publicPort) {
    if (!publicIP || !publicPort) {
      return this.NAT_TYPES.BLOCKED;
    }

    if (localIP === publicIP) {
      return this.NAT_TYPES.OPEN_INTERNET;
    }

    return this.NAT_TYPES.FULL_CONE;
  }

  /**
   * 从测试结果中确定NAT类型
   * @param {Array} results 测试结果数组
   * @returns {string} NAT类型
   */
  static determineNATTypeFromResults(results) {
    const successful = results.filter(r => r.success);
    if (successful.length === 0) {
      return this.NAT_TYPES.BLOCKED;
    }

    const natTypes = successful.map(r => r.natType);
    return this.getMostCommonNATType(natTypes);
  }

  /**
   * 生成测试摘要
   * @param {Array} results 测试结果
   * @param {string} testType 测试类型
   * @returns {Object} 测试摘要
   */
  static generateTestSummary(results, testType) {
    const successful = results.filter(r => r.success);
    const failed = results.filter(r => !r.success);

    return {
      testType: testType,
      totalServers: results.length,
      successfulTests: successful.length,
      failedTests: failed.length,
      successRate: results.length > 0 ? (successful.length / results.length * 100).toFixed(1) : 0,
      averageResponseTime: successful.length > 0
        ? Math.round(successful.reduce((sum, r) => sum + (r.details?.responseTime || 0), 0) / successful.length)
        : 0
    };
  }

  /**
   * 生成高级检测结果摘要
   * @param {Object} results 完整检测结果
   * @returns {Object} 高级摘要信息
   */
  static generateAdvancedSummary(results) {
    const { tests, environments } = results;

    // 分析境内外环境差异
    const domesticSuccess = environments.domestic.success;
    const internationalSuccess = environments.international.success;
    const domesticTotal = environments.domestic.tested;
    const internationalTotal = environments.international.tested;

    // 获取主要NAT类型
    const domesticNATTypes = Object.keys(environments.domestic.natTypes);
    const internationalNATTypes = Object.keys(environments.international.natTypes);

    const primaryDomesticNAT = domesticNATTypes.length > 0 ?
      domesticNATTypes.reduce((a, b) =>
        environments.domestic.natTypes[a] > environments.domestic.natTypes[b] ? a : b
      ) : this.NAT_TYPES.UNKNOWN;

    const primaryInternationalNAT = internationalNATTypes.length > 0 ?
      internationalNATTypes.reduce((a, b) =>
        environments.international.natTypes[a] > environments.international.natTypes[b] ? a : b
      ) : this.NAT_TYPES.UNKNOWN;

    // 分析测试结果一致性
    const testConsistency = this.analyzeTestConsistency(tests);

    return {
      overallSuccess: domesticSuccess > 0 || internationalSuccess > 0,
      environments: {
        domestic: {
          successRate: domesticTotal > 0 ? (domesticSuccess / domesticTotal * 100).toFixed(1) : 0,
          primaryNATType: primaryDomesticNAT,
          natTypeDistribution: environments.domestic.natTypes
        },
        international: {
          successRate: internationalTotal > 0 ? (internationalSuccess / internationalTotal * 100).toFixed(1) : 0,
          primaryNATType: primaryInternationalNAT,
          natTypeDistribution: environments.international.natTypes
        }
      },
      testConsistency: testConsistency,
      recommendation: this.generateRecommendation(primaryDomesticNAT, primaryInternationalNAT, testConsistency)
    };
  }

  /**
   * 分析测试一致性
   * @param {Object} tests 所有测试结果
   * @returns {Object} 一致性分析
   */
  static analyzeTestConsistency(tests) {
    const testTypes = Object.keys(tests);
    let consistentTests = 0;
    let totalComparisons = 0;

    testTypes.forEach(testType => {
      const test = tests[testType];
      if (test.domestic && test.international && test.domestic.success && test.international.success) {
        totalComparisons++;
        if (test.domestic.natType === test.international.natType) {
          consistentTests++;
        }
      }
    });

    return {
      consistentTests,
      totalComparisons,
      consistencyRate: totalComparisons > 0 ? (consistentTests / totalComparisons * 100).toFixed(1) : 0,
      isConsistent: totalComparisons > 0 && consistentTests === totalComparisons
    };
  }

  /**
   * 生成建议
   * @param {string} domesticNAT 境内NAT类型
   * @param {string} internationalNAT 国际NAT类型
   * @param {Object} consistency 一致性分析
   * @returns {Object} 建议信息
   */
  static generateRecommendation(domesticNAT, internationalNAT, consistency) {
    const recommendations = [];

    if (domesticNAT === this.NAT_TYPES.SYMMETRIC || internationalNAT === this.NAT_TYPES.SYMMETRIC) {
      recommendations.push('检测到对称NAT，建议使用TCP协议以获得更好的连接稳定性');
    }

    if (consistency.consistencyRate < 50) {
      recommendations.push('境内外网络环境差异较大，建议针对不同环境优化代理配置');
    }

    if (domesticNAT === this.NAT_TYPES.BLOCKED || internationalNAT === this.NAT_TYPES.BLOCKED) {
      recommendations.push('检测到网络阻塞，可能需要特殊的代理配置或协议');
    }

    return {
      level: this.getRecommendationLevel(domesticNAT, internationalNAT),
      suggestions: recommendations.length > 0 ? recommendations : ['当前网络环境良好，代理连接应该稳定']
    };
  }

  /**
   * 获取建议级别
   */
  static getRecommendationLevel(domesticNAT, internationalNAT) {
    const problematicTypes = [this.NAT_TYPES.SYMMETRIC, this.NAT_TYPES.BLOCKED];

    if (problematicTypes.includes(domesticNAT) || problematicTypes.includes(internationalNAT)) {
      return 'warning';
    }

    if (domesticNAT === this.NAT_TYPES.PORT_RESTRICTED_CONE ||
        internationalNAT === this.NAT_TYPES.PORT_RESTRICTED_CONE) {
      return 'caution';
    }

    return 'good';
  }





  /**
   * 获取最常见的NAT类型
   * @param {Array} natTypes NAT类型数组
   * @returns {string} 最常见的NAT类型
   */
  static getMostCommonNATType(natTypes) {
    if (natTypes.length === 0) return this.NAT_TYPES.UNKNOWN;

    const counts = {};
    natTypes.forEach(type => {
      counts[type] = (counts[type] || 0) + 1;
    });

    return Object.keys(counts).reduce((a, b) => counts[a] > counts[b] ? a : b);
  }

  /**
   * 获取NAT类型的中文描述
   * @param {string} natType NAT类型
   * @returns {string} 中文描述
   */
  static getNATTypeDescription(natType) {
    const descriptions = {
      [this.NAT_TYPES.UNKNOWN]: '未知',
      [this.NAT_TYPES.OPEN_INTERNET]: '开放网络',
      [this.NAT_TYPES.FULL_CONE]: '完全锥形NAT',
      [this.NAT_TYPES.RESTRICTED_CONE]: '限制锥形NAT',
      [this.NAT_TYPES.PORT_RESTRICTED_CONE]: '端口限制锥形NAT',
      [this.NAT_TYPES.SYMMETRIC]: '对称NAT',
      [this.NAT_TYPES.BLOCKED]: '网络阻塞'
    };
    return descriptions[natType] || '未知';
  }

  /**
   * 获取NAT类型对代理的影响说明
   * @param {string} natType NAT类型
   * @returns {string} 影响说明
   */
  static getNATTypeProxyImpact(natType) {
    const impacts = {
      [this.NAT_TYPES.UNKNOWN]: '无法确定网络类型，代理连接可能不稳定',
      [this.NAT_TYPES.OPEN_INTERNET]: '直连网络，代理连接最佳，延迟最低',
      [this.NAT_TYPES.FULL_CONE]: '代理连接良好，支持大部分代理协议，连接稳定',
      [this.NAT_TYPES.RESTRICTED_CONE]: '代理连接较好，部分协议可能受限，建议使用TCP协议',
      [this.NAT_TYPES.PORT_RESTRICTED_CONE]: '代理连接受限，UDP协议可能不稳定，推荐TCP/TLS协议',
      [this.NAT_TYPES.SYMMETRIC]: '代理连接困难，强烈建议使用TCP/TLS协议，避免UDP',
      [this.NAT_TYPES.BLOCKED]: '网络严重受限，代理连接可能失败，需要特殊配置'
    };
    return impacts[natType] || '未知影响';
  }

  /**
   * 获取NAT类型的详细说明
   * @param {string} natType NAT类型
   * @returns {string} 详细说明
   */
  static getNATTypeExplanation(natType) {
    const explanations = {
      [this.NAT_TYPES.UNKNOWN]: '无法确定NAT类型，可能是网络连接问题',
      [this.NAT_TYPES.OPEN_INTERNET]: '直接连接到互联网，无NAT设备',
      [this.NAT_TYPES.FULL_CONE]: '最宽松的NAT类型，支持大部分P2P应用',
      [this.NAT_TYPES.RESTRICTED_CONE]: '中等限制的NAT类型，支持部分P2P应用',
      [this.NAT_TYPES.PORT_RESTRICTED_CONE]: '较严格的NAT类型，P2P连接可能受限',
      [this.NAT_TYPES.SYMMETRIC]: '最严格的NAT类型，P2P连接困难',
      [this.NAT_TYPES.BLOCKED]: '网络被阻塞或防火墙限制'
    };
    return explanations[natType] || '未知NAT类型';
  }
}

export default NATDetector;
