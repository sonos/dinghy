import os
import lldb
import shlex

def connect_command(debugger, command, result, internal_dict):
    connect_url = command
    error = lldb.SBError()
    process = lldb.target.ConnectRemote(lldb.target.GetDebugger().GetListener(), connect_url, None, error)

def set_remote_path(debugger, command, result, internal_dict):
    device_app = command
    error = lldb.SBError()
    lldb.target.modules[0].SetPlatformFileSpec(lldb.SBFileSpec(device_app))

def start(debugger, command, result, internal_dict):
    error = lldb.SBError()
    info = lldb.SBLaunchInfo(shlex.split(command))
    info.SetEnvironmentEntries(["ENV_VAR_PLACEHOLDER"], True)
    proc = lldb.target.Launch(info, error)
    lockedstr = ': Locked'
    if proc.GetState() != lldb.eStateExited:
        print("process left in lldb state: %s"%(debugger.StateAsCString(proc.GetState())))
    if lockedstr in str(error):
        print('\nDevice Locked\n')
        os._exit(254)
    elif proc.GetState() == lldb.eStateStopped:
        thread = proc.GetSelectedThread()
        print(thread)
        for frame in thread:
            print("  %s"%(frame))
        os._exit(-1)
    elif not error.Success():
        print(str(error))
    if proc.exit_state != 0:
        os._exit(proc.exit_state)

