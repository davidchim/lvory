import React, { useState, useRef, useEffect } from 'react';
import { useTranslation } from 'react-i18next';
import TracerouteMap from './TracerouteMap';

const TracerouteTool = () => {
  const { t } = useTranslation();
  const [targetHost, setTargetHost] = useState('');
  const [isTracing, setIsTracing] = useState(false);
  const [showMap, setShowMap] = useState(false);
  const [tracerouteData, setTracerouteData] = useState([]);
  const [traceStats, setTraceStats] = useState(null);
  const [currentHop, setCurrentHop] = useState(0);
  const [traceStatus, setTraceStatus] = useState('');
  const tracerouteRef = useRef(null);

  const copyToClipboard = async (text) => {
    try {
      await navigator.clipboard.writeText(text);
      console.log('IP地址已复制到剪贴板:', text);
    } catch (err) {
      console.error('复制失败:', err);
      const textArea = document.createElement('textarea');
      textArea.value = text;
      document.body.appendChild(textArea);
      textArea.select();
      document.execCommand('copy');
      document.body.removeChild(textArea);
    }
  };

  useEffect(() => {
    if (!window.electron?.traceroute) return;

    const removeStartedListener = window.electron.traceroute.onStarted((data) => {
      console.log('Traceroute started:', data);
      setTraceStatus(`开始追踪到 ${data.target}...`);
      setCurrentHop(0);
    });

    const removeDestinationListener = window.electron.traceroute.onDestination((data) => {
      console.log('Traceroute destination:', data);
      setTraceStatus(`目标地址: ${data.destination}`);
    });

    const removeHopListener = window.electron.traceroute.onHop((hopData) => {
      console.log('Received hop:', hopData);
      setCurrentHop(hopData.hop);
      setTraceStatus(`正在追踪第 ${hopData.hop} 跳: ${hopData.ip}`);

      setTracerouteData(prevData => {
        const newData = [...prevData];
        const existingIndex = newData.findIndex(item => item.hop === hopData.hop);

        if (existingIndex >= 0) {
          newData[existingIndex] = hopData;
        } else {
          newData.push(hopData);
        }

        newData.sort((a, b) => a.hop - b.hop);
        return newData;
      });
    });

    const removeCompleteListener = window.electron.traceroute.onComplete((data) => {
      console.log('Traceroute completed:', data);
      setIsTracing(false);
      setTraceStatus('追踪完成');

      setTracerouteData(prevData => {
        if (prevData.length > 0) {
          const newData = [...prevData];
          newData[newData.length - 1].type = 'destination';
          return newData;
        }
        return prevData;
      });
    });

    const removeErrorListener = window.electron.traceroute.onError((data) => {
      console.error('Traceroute error:', data);
      setIsTracing(false);
      setTraceStatus(`追踪失败: ${data.error}`);
      alert('追踪失败: ' + data.error);
    });

    const removeTimeoutListener = window.electron.traceroute.onTimeout((data) => {
      console.warn('Traceroute timeout:', data);
      setIsTracing(false);
      setTraceStatus('追踪超时');
      alert('追踪超时，请重试');
    });

    return () => {
      removeStartedListener();
      removeDestinationListener();
      removeHopListener();
      removeCompleteListener();
      removeErrorListener();
      removeTimeoutListener();
    };
  }, []);

  const handleStartTrace = async () => {
    setShowMap(false);
    setTracerouteData([]);
    setTraceStats(null);
    setCurrentHop(0);
    setTraceStatus('准备开始追踪...');
    setIsTracing(true);

    try {
      if (window.electron && window.electron.traceroute) {
        const result = await window.electron.traceroute.executeRealtime(targetHost);

        if (!result.success) {
          console.error('Traceroute failed:', result.error);
          alert('追踪失败: ' + result.error);
          setIsTracing(false);
          setTraceStatus('追踪失败');
        }
      } else {
        console.error('Traceroute service not available');
        alert('追踪服务不可用');
        setIsTracing(false);
        setTraceStatus('服务不可用');
      }
    } catch (error) {
      console.error('Traceroute execution failed:', error);
      alert('追踪执行失败: ' + error.message);
      setIsTracing(false);
      setTraceStatus('执行失败');
    }
  };

  const handleStopTrace = async () => {
    try {
      if (window.electron && window.electron.traceroute) {
        const result = await window.electron.traceroute.stop();
        if (result.success) {
          setIsTracing(false);
          setTraceStatus('追踪已停止');
        } else {
          console.error('Stop traceroute failed:', result.error);
          alert('停止追踪失败: ' + result.error);
        }
      }
    } catch (error) {
      console.error('Stop traceroute execution failed:', error);
      alert('停止追踪执行失败: ' + error.message);
    }
  };

  const onTraceComplete = (data) => {
    setTracerouteData(data);
    if (data.length > 0) {
      const avgRtt = data.reduce((sum, hop) => sum + (hop.rtt || 0), 0) / data.length;
      const maxRtt = Math.max(...data.map(hop => hop.rtt || 0));
      const minRtt = Math.min(...data.filter(hop => hop.rtt > 0).map(hop => hop.rtt || Infinity));

      setTraceStats({
        hopCount: data.length,
        avgRtt: avgRtt.toFixed(2),
        maxRtt: maxRtt.toFixed(2),
        minRtt: minRtt === Infinity ? '0' : minRtt.toFixed(2),
        target: targetHost,
        timestamp: new Date().toLocaleString()
      });
    }
  };

  useEffect(() => {
    if (tracerouteData.length > 0) {
      const validRtts = tracerouteData.filter(hop => hop.rtt && hop.rtt > 0).map(hop => hop.rtt);
      if (validRtts.length > 0) {
        const avgRtt = validRtts.reduce((sum, rtt) => sum + rtt, 0) / validRtts.length;
        const maxRtt = Math.max(...validRtts);
        const minRtt = Math.min(...validRtts);

        setTraceStats({
          hopCount: tracerouteData.length,
          avgRtt: avgRtt.toFixed(2),
          maxRtt: maxRtt.toFixed(2),
          minRtt: minRtt.toFixed(2),
          target: targetHost,
          timestamp: new Date().toLocaleString()
        });
      }
    }
  }, [tracerouteData, targetHost]);

  const renderTextView = () => (
    <div className="traceroute-text-view">
      <div className="trace-header">
        {isTracing && (
          <div className="trace-status">
            <div className="status-indicator">
              <span className="status-dot pulsing"></span>
              <span className="status-text">{traceStatus}</span>
            </div>
            {currentHop > 0 && (
              <div className="current-hop">
                当前跳数: {currentHop}
              </div>
            )}
          </div>
        )}
        {traceStats && (
          <div className="trace-stats">
            <div className="stats-info">
              <span className="stat-item">Hops: {traceStats.hopCount}</span>
              <span className="stat-item">Avg RTT: {traceStats.avgRtt}ms</span>
              <span className="stat-item">Min: {traceStats.minRtt}ms</span>
              <span className="stat-item">Max: {traceStats.maxRtt}ms</span>
              <span className="stat-item">Time: {traceStats.timestamp}</span>
            </div>
            {tracerouteData.length > 0 && (
              <button
                className="view-map-button"
                onClick={() => setShowMap(true)}
                disabled={isTracing}
              >
                {t('tools.viewMap')}
              </button>
            )}
          </div>
        )}
      </div>

      <div className="hop-table">
        <div className="hop-table-header">
          <div className="hop-col-num">#</div>
          <div className="hop-col-ip">IP Address</div>
          <div className="hop-col-location">Location</div>
          <div className="hop-col-rtt">Hops RTT</div>
          <div className="hop-col-status">Status</div>
        </div>
        
        <div className="hop-table-body">
          {tracerouteData.map((hop, index) => (
            <div key={index} className={`hop-table-row ${hop.type}`}>
              <div className="hop-col-num">
                {hop.type === 'source' ? 'SRC' : hop.type === 'destination' ? 'DST' : hop.hop}
              </div>
              <div 
                className="hop-col-ip" 
                title={`${hop.ip} (点击复制)`}
                onClick={() => copyToClipboard(hop.ip)}
              >
                {hop.ip}
              </div>
              <div className="hop-col-location" title={`${hop.country}, ${hop.city}`}>
                {hop.country === 'Unknown' ? '-' : `${hop.country}, ${hop.city}`}
              </div>
              <div className="hop-col-rtt">
                {hop.rtt ? hop.rtt.toFixed(2) : '-'}
              </div>
              <div className="hop-col-status">
                <span className={`status-dot ${hop.rtt ? 'status-ok' : 'status-timeout'}`}></span>
                {hop.rtt ? 'OK' : 'Timeout'}
              </div>
            </div>
          ))}
        </div>
      </div>
    </div>
  );

  return (
    <div className="traceroute-tool">
      <div className="tool-control-panel">
        <div className="tool-main-controls">
          <div className="control-group target-group">
            <label htmlFor="target-host">{t('tools.targetHost')}:</label>
            <input
              id="target-host"
              type="text"
              value={targetHost}
              onChange={(e) => setTargetHost(e.target.value)}
              placeholder={t('tools.targetPlaceholder')}
              disabled={isTracing}
              className="target-input"
            />
          </div>

          <button
            onClick={isTracing ? handleStopTrace : handleStartTrace}
            disabled={!targetHost.trim()}
            className={`trace-button ${isTracing ? 'tracing' : ''}`}
          >
            {isTracing ? t('tools.stopTrace') : t('tools.startTrace')}
          </button>
        </div>
      </div>

      <div className="traceroute-content">
        {showMap ? (
          <TracerouteMap
            ref={tracerouteRef}
            targetHost={targetHost}
            setTargetHost={setTargetHost}
            setIsTracing={setIsTracing}
            onTraceComplete={onTraceComplete}
            existingData={tracerouteData}
            onBackToTable={() => setShowMap(false)}
          />
        ) : (
          renderTextView()
        )}
      </div>
    </div>
  );
};

export default TracerouteTool;
