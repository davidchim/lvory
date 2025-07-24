import React, { useState } from 'react';
import { useTranslation } from 'react-i18next';
import NATDetection from './NATDetection';
import TracerouteTool from './TracerouteTool';
import '../../assets/css/tools.css';

const Tools = () => {
  const { t } = useTranslation();
  const [selectedTool, setSelectedTool] = useState('traceroute');

  const tools = [
    {
      id: 'traceroute',
      name: t('tools.traceroute'),
      description: t('tools.tracerouteDescription'),
      component: TracerouteTool
    },
    {
      id: 'nat',
      name: t('tools.natDetection'),
      description: t('tools.natDetectionDescription'),
      component: NATDetection
    }
  ];

  const selectedToolData = tools.find(tool => tool.id === selectedTool);
  const ToolComponent = selectedToolData?.component;

  return (
    <div className="tools-container">
      <div className="tools-content">
        <div className="tool-tabs">
          {tools.map(tool => (
            <button
              key={tool.id}
              className={`tool-tab ${selectedTool === tool.id ? 'active' : ''}`}
              onClick={() => setSelectedTool(tool.id)}
            >
              {tool.name}
            </button>
          ))}
        </div>

        <div className="tool-view-area">
          {ToolComponent && <ToolComponent />}
        </div>
      </div>
    </div>
  );
};

export default Tools;