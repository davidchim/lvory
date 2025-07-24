import React, { useState, useRef } from 'react';
import { useTranslation } from 'react-i18next';
import NATDetector from '../../services/nat/NATDetector';

const NATDetection = () => {
  const { t } = useTranslation();
  const [isDetecting, setIsDetecting] = useState(false);
  const [natResults, setNatResults] = useState(null);
  const [detectionStatus, setDetectionStatus] = useState('');
  const [progress, setProgress] = useState(0);
  const [currentPhase, setCurrentPhase] = useState('');
  const [testMode, setTestMode] = useState('all'); // 'all' | 'single'
  const [selectedTest, setSelectedTest] = useState(NATDetector.TEST_TYPES.EIM);
  const [testResults, setTestResults] = useState({});
  const [environmentResults, setEnvironmentResults] = useState({
    domestic: { tested: 0, success: 0, natTypes: {} },
    international: { tested: 0, success: 0, natTypes: {} }
  });
  const abortControllerRef = useRef(null);

  const handleStartDetection = async () => {
    setIsDetecting(true);
    setNatResults(null);
    setDetectionStatus(t('tools.detecting'));
    setProgress(0);

    // 重置测试结果
    setTestResults({
      [NATDetector.TEST_TYPES.EIM]: { status: 'pending', domestic: null, international: null },
      [NATDetector.TEST_TYPES.ADF]: { status: 'pending', domestic: null, international: null },
      [NATDetector.TEST_TYPES.APDF]: { status: 'pending', domestic: null, international: null },
      [NATDetector.TEST_TYPES.SYMMETRIC]: { status: 'pending', domestic: null, international: null }
    });
    setEnvironmentResults({
      domestic: { tested: 0, success: 0, natTypes: {} },
      international: { tested: 0, success: 0, natTypes: {} }
    });

    abortControllerRef.current = new AbortController();

    try {
      if (testMode === 'all') {
        const result = await NATDetector.performFullDetection((progressData) => {
          setProgress(progressData.progress);
          setCurrentPhase(progressData.phase);

          if (progressData.currentServer) {
            setDetectionStatus(`${t('tools.testing')}: ${progressData.currentServer}`);
          } else {
            switch (progressData.phase) {
              case 'domestic':
                setDetectionStatus(t('tools.testingDomestic'));
                break;
              case 'international':
                setDetectionStatus(t('tools.testingInternational'));
                break;
              case 'completed':
                setDetectionStatus(t('tools.testCompleted'));
                break;
              default:
                setDetectionStatus(t('tools.detecting'));
            }
          }

          // 更新实时测试结果
          if (progressData.testResults) {
            setTestResults(progressData.testResults);
          }
          if (progressData.environmentResults) {
            setEnvironmentResults(progressData.environmentResults);
          }
        });

        setNatResults(result);
        setDetectionStatus('');
      } else {
        setDetectionStatus(t('tools.runningSingleTest'));
        const result = await NATDetector.performSingleTest(selectedTest);
        setNatResults(result);
        setDetectionStatus('');
      }
    } catch (error) {
      console.error('NAT检测错误:', error);
      setDetectionStatus(`${t('tools.testError')}: ${error.message}`);
      setNatResults({ success: false, error: error.message });
    } finally {
      setIsDetecting(false);
      setProgress(100);
      abortControllerRef.current = null;
    }
  };

  const handleStopDetection = () => {
    if (abortControllerRef.current) {
      abortControllerRef.current.abort();
      abortControllerRef.current = null;
    }
    setIsDetecting(false);
    setDetectionStatus(t('tools.stopDetection'));
    setProgress(0);
  };

  const handleTestModeChange = (mode) => {
    if (!isDetecting) {
      setTestMode(mode);
    }
  };

  const handleTestTypeChange = (testType) => {
    if (!isDetecting) {
      setSelectedTest(testType);
    }
  };



  const getNATTypeColor = (natType) => {
    switch (natType) {
      case NATDetector.NAT_TYPES.OPEN_INTERNET:
        return '#38a169'; // 绿色
      case NATDetector.NAT_TYPES.FULL_CONE:
        return '#38a169'; // 绿色
      case NATDetector.NAT_TYPES.RESTRICTED_CONE:
        return '#d69e2e'; // 黄色
      case NATDetector.NAT_TYPES.PORT_RESTRICTED_CONE:
        return '#d69e2e'; // 黄色
      case NATDetector.NAT_TYPES.SYMMETRIC:
        return '#e53e3e'; // 红色
      case NATDetector.NAT_TYPES.BLOCKED:
        return '#e53e3e'; // 红色
      default:
        return '#718096'; // 灰色
    }
  };

  const getRecommendationColor = (level) => {
    switch (level) {
      case 'good':
        return '#38a169';
      case 'caution':
        return '#d69e2e';
      case 'warning':
        return '#e53e3e';
      default:
        return '#718096';
    }
  };

  const renderProfessionalResults = () => {
    if (!natResults) {
      return null;
    }

    if (!natResults.success) {
      return (
        <div className="nat-error">
          <div className="error-message">{t('tools.testError')}: {natResults.error}</div>
        </div>
      );
    }

    // 如果是完整检测结果
    if (natResults.summary && natResults.environments) {
      return (
        <div className="professional-nat-results">
          {/* 环境对比摘要 */}
          <div className="environment-comparison">
            <div className="comparison-header">
              <h4>{t('tools.environmentComparison')}</h4>
            </div>
            <div className="comparison-grid">
              <div className="environment-card domestic">
                <div className="env-header">
                  <h5>{t('tools.domesticEnvironment')}</h5>
                </div>
                <div className="env-stats">
                  <div className="stat-item">
                    <span className="stat-label">{t('tools.successRate')}</span>
                    <span className="stat-value">{natResults.summary.environments.domestic.successRate}%</span>
                  </div>
                  <div className="stat-item">
                    <span className="stat-label">{t('tools.primaryNATType')}</span>
                    <span className="stat-value nat-type" style={{
                      color: getNATTypeColor(natResults.summary.environments.domestic.primaryNATType)
                    }}>
                      {NATDetector.getNATTypeDescription(natResults.summary.environments.domestic.primaryNATType)}
                    </span>
                  </div>
                </div>
              </div>

              <div className="environment-card international">
                <div className="env-header">
                  <h5>{t('tools.internationalEnvironment')}</h5>
                </div>
                <div className="env-stats">
                  <div className="stat-item">
                    <span className="stat-label">{t('tools.successRate')}</span>
                    <span className="stat-value">{natResults.summary.environments.international.successRate}%</span>
                  </div>
                  <div className="stat-item">
                    <span className="stat-label">{t('tools.primaryNATType')}</span>
                    <span className="stat-value nat-type" style={{
                      color: getNATTypeColor(natResults.summary.environments.international.primaryNATType)
                    }}>
                      {NATDetector.getNATTypeDescription(natResults.summary.environments.international.primaryNATType)}
                    </span>
                  </div>
                </div>
              </div>
            </div>
          </div>

          {/* 测试一致性 */}
          <div className="test-consistency">
            <div className="consistency-header">
              <h4>{t('tools.testConsistency')}</h4>
            </div>
            <div className="consistency-content">
              <div className="consistency-rate">
                <span className="rate-label">{t('tools.consistencyRate')}</span>
                <span className="rate-value">{natResults.summary.testConsistency.consistencyRate}%</span>
              </div>
              <div className="consistency-indicator">
                <div
                  className="indicator-bar"
                  style={{
                    width: `${natResults.summary.testConsistency.consistencyRate}%`,
                    backgroundColor: natResults.summary.testConsistency.isConsistent ? '#38a169' : '#d69e2e'
                  }}
                ></div>
              </div>
            </div>
          </div>

          {/* 建议 */}
          {natResults.summary.recommendation && (
            <div className="recommendations">
              <div className="recommendations-header">
                <h4>{t('tools.recommendations')}</h4>
                <span
                  className="recommendation-level"
                  style={{ color: getRecommendationColor(natResults.summary.recommendation.level) }}
                >
                  {t(`tools.recommendation${natResults.summary.recommendation.level.charAt(0).toUpperCase() + natResults.summary.recommendation.level.slice(1)}`)}
                </span>
              </div>
              <div className="recommendations-list">
                {natResults.summary.recommendation.suggestions.map((suggestion, index) => (
                  <div key={index} className="recommendation-item">
                    <span className="recommendation-text">{suggestion}</span>
                  </div>
                ))}
              </div>
            </div>
          )}
        </div>
      );
    }

    // 如果是单项测试结果
    return (
      <div className="single-test-results">
        <div className="test-header">
          <h4>{t('tools.testResults')}</h4>
          <span className="test-type">{natResults.testType}</span>
        </div>
        <div className="test-summary">
          <div className="summary-item">
            <span className="summary-label">{t('tools.successRate')}</span>
            <span className="summary-value">{natResults.summary.successRate}%</span>
          </div>
          <div className="summary-item">
            <span className="summary-label">{t('tools.primaryNATType')}</span>
            <span className="summary-value nat-type" style={{ color: getNATTypeColor(natResults.natType) }}>
              {NATDetector.getNATTypeDescription(natResults.natType)}
            </span>
          </div>
        </div>
      </div>
    );
  };



  return (
    <div className="professional-nat-container">
      {/* 控制面板 - 始终显示在顶部 */}
      <div className="nat-control-panel">
        <div className="control-header">
          <h3>{t('tools.professionalNATDetection')}</h3>
          <p>{t('tools.professionalNATDescription')}</p>
        </div>

        <div className="control-options">
          {/* 测试模式选择 */}
          <div className="test-mode-selector">
            <label>{t('tools.testMode')}</label>
            <div className="mode-buttons">
              <button
                className={`mode-button ${testMode === 'all' ? 'active' : ''}`}
                onClick={() => handleTestModeChange('all')}
                disabled={isDetecting}
              >
                {t('tools.fullTest')}
              </button>
              <button
                className={`mode-button ${testMode === 'single' ? 'active' : ''}`}
                onClick={() => handleTestModeChange('single')}
                disabled={isDetecting}
              >
                {t('tools.singleTest')}
              </button>
            </div>
          </div>

          {/* 单项测试选择 */}
          {testMode === 'single' && (
            <div className="test-type-selector">
              <label>{t('tools.selectTestType')}</label>
              <select
                value={selectedTest}
                onChange={(e) => handleTestTypeChange(e.target.value)}
                disabled={isDetecting}
                className="test-select"
              >
                <option value={NATDetector.TEST_TYPES.EIM}>{t('tools.eimTest')}</option>
                <option value={NATDetector.TEST_TYPES.ADF}>{t('tools.adfTest')}</option>
                <option value={NATDetector.TEST_TYPES.APDF}>{t('tools.apdfTest')}</option>
                <option value={NATDetector.TEST_TYPES.SYMMETRIC}>{t('tools.symmetricTest')}</option>
              </select>
            </div>
          )}

          {/* 控制按钮 */}
          <div className="control-actions">
            <button
              onClick={isDetecting ? handleStopDetection : handleStartDetection}
              className={`detection-button ${isDetecting ? 'detecting' : ''}`}
              disabled={false}
            >
              {isDetecting ? t('tools.stopDetection') : t('tools.startDetection')}
            </button>
          </div>
        </div>

        {/* 进度和状态 */}
        {(isDetecting || detectionStatus) && (
          <div className="detection-progress">
            {isDetecting && (
              <div className="progress-bar">
                <div
                  className="progress-fill"
                  style={{ width: `${progress}%` }}
                ></div>
              </div>
            )}
            <div className="status-indicator">
              <span className={`status-dot ${isDetecting ? 'pulsing' : ''}`}></span>
              <span className="status-text">{detectionStatus}</span>
            </div>
          </div>
        )}
      </div>

      {/* 结果显示区域 */}
      {natResults && (
        <div className="nat-results-section">
          <div className="results-header">
            <h4>{t('tools.testResults')}</h4>
            <button
              onClick={() => {
                setNatResults(null);
                setDetectionStatus('');
                setProgress(0);
              }}
              className="reset-button"
            >
              {t('tools.startDetection')}
            </button>
          </div>
          <div className="results-content">
            {renderProfessionalResults()}
          </div>
        </div>
      )}
    </div>
  );
};

export default NATDetection;
