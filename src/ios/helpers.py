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

def run_command(debugger, command, result, internal_dict):
    args = command.split('--',1)
    args_arr = []
    if len(args) > 1:
        args_arr = shlex.split(args[1])
    else:
        args_arr = shlex.split('')
    error = lldb.SBError()
    lldb.target.Launch(lldb.SBLaunchInfo(args_arr), error)
    lockedstr = ': Locked'
    if lockedstr in str(error):
       print('\nDevice Locked\n')
       os._exit(254)
    else:
       print(str(error))


