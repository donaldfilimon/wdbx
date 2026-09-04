'use client';
import { useState, type ReactNode } from 'react';
export function VirtualList<T>({items,rowHeight,renderItem}:{items:T[];rowHeight:number;renderItem:(item:T)=>ReactNode}) {
 const [scroll,setScroll]=useState(0);const height=Math.min(600,items.length*rowHeight);const first=Math.max(0,Math.min(items.length-1,Math.floor(scroll/rowHeight)-2));const last=Math.min(items.length,first+Math.ceil(height/rowHeight)+5);
 return <div className="virtual-list" style={{height,overflow:'auto'}} onScroll={e=>setScroll(e.currentTarget.scrollTop)}><div style={{height:items.length*rowHeight,position:'relative'}}>{items.slice(first,last).map((item,index)=><div key={first+index} style={{position:'absolute',top:(first+index)*rowHeight,left:0,right:0,height:rowHeight,paddingBottom:10}}>{renderItem(item)}</div>)}</div></div>;
}
