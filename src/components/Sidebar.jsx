import React, { useState, useEffect } from 'react';
import '../assets/css/sidebar.css';
import SystemStatus from './SystemStatus';
import logoSvg from '../../resource/icon/logo.svg';

const Sidebar = ({ activeItem, onItemClick, profilesCount, isMinimized }) => {
  // 为菜单项添加点击涟漪效果
  const [rippleStyle, setRippleStyle] = useState({ top: '0px', left: '0px', display: 'none' });
  
  // 处理菜单项点击并触发涟漪效果
  const handleItemClick = (item, e) => {
    if (!e) return;
    
    const rect = e.currentTarget.getBoundingClientRect();
    const x = e.clientX - rect.left;
    const y = e.clientY - rect.top;
    
    setRippleStyle({
      top: `${y}px`,
      left: `${x}px`,
      display: 'block'
    });
    
    // 触发父组件的点击事件
    onItemClick(item);
    
    // 300ms后隐藏涟漪效果
    setTimeout(() => {
      setRippleStyle({ ...rippleStyle, display: 'none' });
    }, 300);
  };

  return (
    <div className={`sidebar ${isMinimized ? 'minimized' : ''}`}>
      <div className="logo">
        <img src={logoSvg} alt="LVORY Logo" className="logo-image" />
        {!isMinimized && <h2>LVORY</h2>}
      </div>
      
      <div className="main-menu">
        <ul>
          <button
            className={`menu-item ${activeItem === 'dashboard' ? 'active' : ''}`}
            onClick={(e) => handleItemClick('dashboard', e)}
            title={isMinimized ? 'Dashboard' : ''}
          >
            <span className="icon home-icon"></span>
            {!isMinimized && <span>Dashboard</span>}
            <span className="ripple" style={activeItem === 'dashboard' ? rippleStyle : { display: 'none' }}></span>
          </button>
          <button
            className={`menu-item ${activeItem === 'activity' ? 'active' : ''}`}
            onClick={(e) => handleItemClick('activity', e)}
            title={isMinimized ? 'Activity' : ''}
          >
            <span className="icon activity-icon"></span>
            {!isMinimized && <span>Activity</span>}
            <span className="ripple" style={activeItem === 'activity' ? rippleStyle : { display: 'none' }}></span>
          </button>
          <button
            className={`menu-item ${activeItem === 'profiles' ? 'active' : ''}`}
            onClick={(e) => handleItemClick('profiles', e)}
            title={isMinimized ? `Profiles (${profilesCount || 0})` : ''}
          >
            <span className="icon profiles-icon"></span>
            {!isMinimized && <span>Profiles</span>}
            {!isMinimized && profilesCount > 0 && <span className="badge">{profilesCount}</span>}
            {isMinimized && profilesCount > 0 && <span className="badge minimized-badge">{profilesCount}</span>}
            <span className="ripple" style={activeItem === 'profiles' ? rippleStyle : { display: 'none' }}></span>
          </button>
          <button
            className={`menu-item ${activeItem === 'tools' ? 'active' : ''}`}
            onClick={(e) => handleItemClick('tools', e)}
            title={isMinimized ? 'Tools' : ''}
          >
            <span className="icon tools-icon"></span>
            {!isMinimized && <span>Tools</span>}
            <span className="ripple" style={activeItem === 'tools' ? rippleStyle : { display: 'none' }}></span>
          </button>
          <button
            className={`menu-item ${activeItem === 'settings' ? 'active' : ''}`}
            onClick={(e) => handleItemClick('settings', e)}
            title={isMinimized ? 'Settings' : ''}
          >
            <span className="icon settings-icon"></span>
            {!isMinimized && <span>Settings</span>}
            <span className="ripple" style={activeItem === 'settings' ? rippleStyle : { display: 'none' }}></span>
          </button>
        </ul>
      </div>

      {!isMinimized && <SystemStatus />}
    </div>
  );
};

export default Sidebar; 