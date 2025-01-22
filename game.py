1. #game.py
2: # -------
3: # Licensing Information:  You are free to use or extend these projects for
4: # educational purposes provided that (1) you do not distribute or publish
5: # solutions, (2) you retain this notice, and (3) you provide clear
6: # attribution to UC Berkeley, including a link to http://ai.berkeley.edu.
7: # 
8: # Attribution Information: The Pacman AI projects were developed at UC Berkeley.
9: # The core projects and autograders were primarily created by John DeNero
10: # (denero@cs.berkeley.edu) and Dan Klein (klein@cs.berkeley.edu).
11: # Student side autograding was added by Brad Miller, Nick Hay, and
12: # Pieter Abbeel (pabbeel@cs.berkeley.edu).
13: 
14: 
15: # game.py
16: # -------
17: # Licensing Information: Please do not distribute or publish solutions to this
18: # project. You are free to use and extend these projects for educational
19: # purposes. The Pacman AI projects were developed at UC Berkeley, primarily by
20: # John DeNero (denero@cs.berkeley.edu) and Dan Klein (klein@cs.berkeley.edu).
21: # For more info, see http://inst.eecs.berkeley.edu/~cs188/sp09/pacman.html
22: 
23: from util import *
24: import time, os
25: import traceback
26: import sys
27: 
28: #######################
29: # Parts worth reading #
30: #######################
31: 
32: class Agent:
33:     """
34:     An agent must define a getAction method, but may also define the
35:     following methods which will be called if they exist:
36: 
37:     def registerInitialState(self, state): # inspects the starting state
38:     """
39:     def __init__(self, index=0):
40:         self.index = index
41: 
42:     def getAction(self, state):
43:         """
44:         The Agent will receive a GameState (from either {pacman, capture, sonar}.py) and
45:         must return an action from Directions.{North, South, East, West, Stop}
46:         """
47:         raiseNotDefined()
48: 
49: class Directions:
50:     NORTH = 'North'
51:     SOUTH = 'South'
52:     EAST = 'East'
53:     WEST = 'West'
54:     STOP = 'Stop'
55: 
56:     LEFT =       {NORTH: WEST,
57:                    SOUTH: EAST,
58:                    EAST:  NORTH,
59:                    WEST:  SOUTH,
60:                    STOP:  STOP}
61: 
62:     RIGHT =      dict([(y,x) for x, y in LEFT.items()])
63: 
64:     REVERSE = {NORTH: SOUTH,
65:                SOUTH: NORTH,
66:                EAST: WEST,
67:                WEST: EAST,
68:                STOP: STOP}
69: 
70: class Configuration:
71:     """
72:     A Configuration holds the (x,y) coordinate of a character, along with its
73:     traveling direction.
74: 
75:     The convention for positions, like a graph, is that (0,0) is the lower left corner, x increases
76:     horizontally and y increases vertically.  Therefore, north is the direction of increasing y, or (0,1).
77:     """
78: 
79:     def __init__(self, pos, direction):
80:         self.pos = pos
81:         self.direction = direction
82: 
83:     def getPosition(self):
84:         return (self.pos)
85: 
86:     def getDirection(self):
87:         return self.direction
88: 
89:     def isInteger(self):
90:         x,y = self.pos
91:         return x == int(x) and y == int(y)
92: 
93:     def __eq__(self, other):
94:         if other == None: return False
95:         return (self.pos == other.pos and self.direction == other.direction)
96: 
97:     def __hash__(self):
98:         x = hash(self.pos)
99:         y = hash(self.direction)
100:         return hash(x + 13 * y)
101: 
102:     def __str__(self):
103:         return "(x,y)="+str(self.pos)+", "+str(self.direction)
104: 
105:     def generateSuccessor(self, vector):
106:         """
107:         Generates a new configuration reached by translating the current
108:         configuration by the action vector.  This is a low-level call and does
109:         not attempt to respect the legality of the movement.
110: 
111:         Actions are movement vectors.
112:         """
113:         x, y= self.pos
114:         dx, dy = vector
115:         direction = Actions.vectorToDirection(vector)
116:         if direction == Directions.STOP:
117:             direction = self.direction # There is no stop direction
118:         return Configuration((x + dx, y+dy), direction)
119: 
120: class AgentState:
121:     """
122:     AgentStates hold the state of an agent (configuration, speed, scared, etc).
123:     """
124: 
125:     def __init__( self, startConfiguration, isPacman ):
126:         self.start = startConfiguration
127:         self.configuration = startConfiguration
128:         self.isPacman = isPacman
129:         self.scaredTimer = 0
130:         self.numCarrying = 0
131:         self.numReturned = 0
132: 
133:     def __str__( self ):
134:         if self.isPacman:
135:             return "Pacman: " + str( self.configuration )
136:         else:
137:             return "Ghost: " + str( self.configuration )
138: 
139:     def __eq__( self, other ):
140:         if other == None:
141:             return False
142:         return self.configuration == other.configuration and self.scaredTimer == other.scaredTimer
143: 
144:     def __hash__(self):
145:         return hash(hash(self.configuration) + 13 * hash(self.scaredTimer))
146: 
147:     def copy( self ):
148:         state = AgentState( self.start, self.isPacman )
149:         state.configuration = self.configuration
150:         state.scaredTimer = self.scaredTimer
151:         state.numCarrying = self.numCarrying
152:         state.numReturned = self.numReturned
153:         return state
154: 
155:     def getPosition(self):
156:         if self.configuration == None: return None
157:         return self.configuration.getPosition()
158: 
159:     def getDirection(self):
160:         return self.configuration.getDirection()
161: 
162: class Grid:
163:     """
164:     A 2-dimensional array of objects backed by a list of lists.  Data is accessed
165:     via grid[x][y] where (x,y) are positions on a Pacman map with x horizontal,
166:     y vertical and the origin (0,0) in the bottom left corner.
167: 
168:     The __str__ method constructs an output that is oriented like a pacman board.
169:     """
170:     def __init__(self, width, height, initialValue=False, bitRepresentation=None):
171:         if initialValue not in [False, True]: raise Exception('Grids can only contain booleans')
172:         self.CELLS_PER_INT = 30
173: 
174:         self.width = width
175:         self.height = height
176:         self.data = [[initialValue for y in range(height)] for x in range(width)]
177:         if bitRepresentation:
178:             self._unpackBits(bitRepresentation)
179: 
180:     def __getitem__(self, i):
181:         return self.data[i]
182: 
183:     def __setitem__(self, key, item):
184:         self.data[key] = item
185: 
186:     def __str__(self):
187:         out = [[str(self.data[x][y])[0] for x in range(self.width)] for y in range(self.height)]
188:         out.reverse()
189:         return '\n'.join([''.join(x) for x in out])
190: 
191:     def __eq__(self, other):
192:         if other == None: return False
193:         return self.data == other.data
194: 
195:     def __hash__(self):
196:         # return hash(str(self))
197:         base = 1
198:         h = 0
199:         for l in self.data:
200:             for i in l:
201:                 if i:
202:                     h += base
203:                 base *= 2
204:         return hash(h)
205: 
206:     def copy(self):
207:         g = Grid(self.width, self.height)
208:         g.data = [x[:] for x in self.data]
209:         return g
210: 
211:     def deepCopy(self):
212:         return self.copy()
213: 
214:     def shallowCopy(self):
215:         g = Grid(self.width, self.height)
216:         g.data = self.data
217:         return g
218: 
219:     def count(self, item =True ):
220:         return sum([x.count(item) for x in self.data])
221: 
222:     def asList(self, key = True):
223:         list = []
224:         for x in range(self.width):
225:             for y in range(self.height):
226:                 if self[x][y] == key: list.append( (x,y) )
227:         return list
228: 
229:     def packBits(self):
230:         """
231:         Returns an efficient int list representation
232: 
233:         (width, height, bitPackedInts...)
234:         """
235:         bits = [self.width, self.height]
236:         currentInt = 0
237:         for i in range(self.height * self.width):
238:             bit = self.CELLS_PER_INT - (i % self.CELLS_PER_INT) - 1
239:             x, y = self._cellIndexToPosition(i)
240:             if self[x][y]:
241:                 currentInt += 2 ** bit
242:             if (i + 1) % self.CELLS_PER_INT == 0:
243:                 bits.append(currentInt)
244:                 currentInt = 0
245:         bits.append(currentInt)
246:         return tuple(bits)
247: 
248:     def _cellIndexToPosition(self, index):
249:         x = index / self.height
250:         y = index % self.height
251:         return x, y
252: 
253:     def _unpackBits(self, bits):
254:         """
255:         Fills in data from a bit-level representation
256:         """
257:         cell = 0
258:         for packed in bits:
259:             for bit in self._unpackInt(packed, self.CELLS_PER_INT):
260:                 if cell == self.width * self.height: break
261:                 x, y = self._cellIndexToPosition(cell)
262:                 self[x][y] = bit
263:                 cell += 1
264: 
265:     def _unpackInt(self, packed, size):
266:         bools = []
267:         if packed < 0: raise ValueError, "must be a positive integer"
268:         for i in range(size):
269:             n = 2 ** (self.CELLS_PER_INT - i - 1)
270:             if packed >= n:
271:                 bools.append(True)
272:                 packed -= n
273:             else:
274:                 bools.append(False)
275:         return bools
276: 
277: def reconstituteGrid(bitRep):
278:     if type(bitRep) is not type((1,2)):
279:         return bitRep
280:     width, height = bitRep[:2]
281:     return Grid(width, height, bitRepresentation= bitRep[2:])
282: 
283: ####################################
284: # Parts you shouldn't have to read #
285: ####################################
286: 
287: class Actions:
288:     """
289:     A collection of static methods for manipulating move actions.
290:     """
291:     # Directions
292:     _directions = {Directions.NORTH: (0, 1),
293:                    Directions.SOUTH: (0, -1),
294:                    Directions.EAST:  (1, 0),
295:                    Directions.WEST:  (-1, 0),
296:                    Directions.STOP:  (0, 0)}
297: 
298:     _directionsAsList = _directions.items()
299: 
300:     TOLERANCE = .001
301: 
302:     def reverseDirection(action):
303:         if action == Directions.NORTH:
304:             return Directions.SOUTH
305:         if action == Directions.SOUTH:
306:             return Directions.NORTH
307:         if action == Directions.EAST:
308:             return Directions.WEST
309:         if action == Directions.WEST:
310:             return Directions.EAST
311:         return action
312:     reverseDirection = staticmethod(reverseDirection)
313: 
314:     def vectorToDirection(vector):
315:         dx, dy = vector
316:         if dy > 0:
317:             return Directions.NORTH
318:         if dy < 0:
319:             return Directions.SOUTH
320:         if dx < 0:
321:             return Directions.WEST
322:         if dx > 0:
323:             return Directions.EAST
324:         return Directions.STOP
325:     vectorToDirection = staticmethod(vectorToDirection)
326: 
327:     def directionToVector(direction, speed = 1.0):
328:         dx, dy =  Actions._directions[direction]
329:         return (dx * speed, dy * speed)
330:     directionToVector = staticmethod(directionToVector)
331: 
332:     def getPossibleActions(config, walls):
333:         possible = []
334:         x, y = config.pos
335:         x_int, y_int = int(x + 0.5), int(y + 0.5)
336: 
337:         # In between grid points, all agents must continue straight
338:         if (abs(x - x_int) + abs(y - y_int)  > Actions.TOLERANCE):
339:             return [config.getDirection()]
340: 
341:         for dir, vec in Actions._directionsAsList:
342:             dx, dy = vec
343:             next_y = y_int + dy
344:             next_x = x_int + dx
345:             if not walls[next_x][next_y]: possible.append(dir)
346: 
347:         return possible
348: 
349:     getPossibleActions = staticmethod(getPossibleActions)
350: 
351:     def getLegalNeighbors(position, walls):
352:         x,y = position
353:         x_int, y_int = int(x + 0.5), int(y + 0.5)
354:         neighbors = []
355:         for dir, vec in Actions._directionsAsList:
356:             dx, dy = vec
357:             next_x = x_int + dx
358:             if next_x < 0 or next_x == walls.width: continue
359:             next_y = y_int + dy
360:             if next_y < 0 or next_y == walls.height: continue
361:             if not walls[next_x][next_y]: neighbors.append((next_x, next_y))
362:         return neighbors
363:     getLegalNeighbors = staticmethod(getLegalNeighbors)
364: 
365:     def getSuccessor(position, action):
366:         dx, dy = Actions.directionToVector(action)
367:         x, y = position
368:         return (x + dx, y + dy)
369:     getSuccessor = staticmethod(getSuccessor)
370: 
371: class GameStateData:
372:     """
373: 
374:     """
375:     def __init__( self, prevState = None ):
376:         """
377:         Generates a new data packet by copying information from its predecessor.
378:         """
379:         if prevState != None:
380:             self.food = prevState.food.shallowCopy()
381:             self.capsules = prevState.capsules[:]
382:             self.agentStates = self.copyAgentStates( prevState.agentStates )
383:             self.layout = prevState.layout
384:             self._eaten = prevState._eaten
385:             self.score = prevState.score
386: 
387:         self._foodEaten = None
388:         self._foodAdded = None
389:         self._capsuleEaten = None
390:         self._agentMoved = None
391:         self._lose = False
392:         self._win = False
393:         self.scoreChange = 0
394: 
395:     def deepCopy( self ):
396:         state = GameStateData( self )
397:         state.food = self.food.deepCopy()
398:         state.layout = self.layout.deepCopy()
399:         state._agentMoved = self._agentMoved
400:         state._foodEaten = self._foodEaten
401:         state._foodAdded = self._foodAdded
402:         state._capsuleEaten = self._capsuleEaten
403:         return state
404: 
405:     def copyAgentStates( self, agentStates ):
406:         copiedStates = []
407:         for agentState in agentStates:
408:             copiedStates.append( agentState.copy() )
409:         return copiedStates
410: 
411:     def __eq__( self, other ):
412:         """
413:         Allows two states to be compared.
414:         """
415:         if other == None: return False
416:         # TODO Check for type of other
417:         if not self.agentStates == other.agentStates: return False
418:         if not self.food == other.food: return False
419:         if not self.capsules == other.capsules: return False
420:         if not self.score == other.score: return False
421:         return True
422: 
423:     def __hash__( self ):
424:         """
425:         Allows states to be keys of dictionaries.
426:         """
427:         for i, state in enumerate( self.agentStates ):
428:             try:
429:                 int(hash(state))
430:             except TypeError, e:
431:                 print e
432:                 #hash(state)
433:         return int((hash(tuple(self.agentStates)) + 13*hash(self.food) + 113* hash(tuple(self.capsules)) + 7 * hash(self.score)) % 1048575 )
434: 
435:     def __str__( self ):
436:         width, height = self.layout.width, self.layout.height
437:         map = Grid(width, height)
438:         if type(self.food) == type((1,2)):
439:             self.food = reconstituteGrid(self.food)
440:         for x in range(width):
441:             for y in range(height):
442:                 food, walls = self.food, self.layout.walls
443:                 map[x][y] = self._foodWallStr(food[x][y], walls[x][y])
444: 
445:         for agentState in self.agentStates:
446:             if agentState == None: continue
447:             if agentState.configuration == None: continue
448:             x,y = [int( i ) for i in nearestPoint( agentState.configuration.pos )]
449:             agent_dir = agentState.configuration.direction
450:             if agentState.isPacman:
451:                 map[x][y] = self._pacStr( agent_dir )
452:             else:
453:                 map[x][y] = self._ghostStr( agent_dir )
454: 
455:         for x, y in self.capsules:
456:             map[x][y] = 'o'
457: 
458:         return str(map) + ("\nScore: %d\n" % self.score)
459: 
460:     def _foodWallStr( self, hasFood, hasWall ):
461:         if hasFood:
462:             return '.'
463:         elif hasWall:
464:             return '%'
465:         else:
466:             return ' '
467: 
468:     def _pacStr( self, dir ):
469:         if dir == Directions.NORTH:
470:             return 'v'
471:         if dir == Directions.SOUTH:
472:             return '^'
473:         if dir == Directions.WEST:
474:             return '>'
475:         return '<'
476: 
477:     def _ghostStr( self, dir ):
478:         return 'G'
479:         if dir == Directions.NORTH:
480:             return 'M'
481:         if dir == Directions.SOUTH:
482:             return 'W'
483:         if dir == Directions.WEST:
484:             return '3'
485:         return 'E'
486: 
487:     def initialize( self, layout, numGhostAgents ):
488:         """
489:         Creates an initial game state from a layout array (see layout.py).
490:         """
491:         self.food = layout.food.copy()
492:         #self.capsules = []
493:         self.capsules = layout.capsules[:]
494:         self.layout = layout
495:         self.score = 0
496:         self.scoreChange = 0
497: 
498:         self.agentStates = []
499:         numGhosts = 0
500:         for isPacman, pos in layout.agentPositions:
501:             if not isPacman:
502:                 if numGhosts == numGhostAgents: continue # Max ghosts reached already
503:                 else: numGhosts += 1
504:             self.agentStates.append( AgentState( Configuration( pos, Directions.STOP), isPacman) )
505:         self._eaten = [False for a in self.agentStates]
506: 
507: try:
508:     import boinc
509:     _BOINC_ENABLED = True
510: except:
511:     _BOINC_ENABLED = False
512: 
513: class Game:
514:     """
515:     The Game manages the control flow, soliciting actions from agents.
516:     """
517: 
518:     def __init__( self, agents, display, rules, startingIndex=0, muteAgents=False, catchExceptions=False ):
519:         self.agentCrashed = False
520:         self.agents = agents
521:         self.display = display
522:         self.rules = rules
523:         self.startingIndex = startingIndex
524:         self.gameOver = False
525:         self.muteAgents = muteAgents
526:         self.catchExceptions = catchExceptions
527:         self.moveHistory = []
528:         self.totalAgentTimes = [0 for agent in agents]
529:         self.totalAgentTimeWarnings = [0 for agent in agents]
530:         self.agentTimeout = False
531:         import cStringIO
532:         self.agentOutput = [cStringIO.StringIO() for agent in agents]
533: 
534:     def getProgress(self):
535:         if self.gameOver:
536:             return 1.0
537:         else:
538:             return self.rules.getProgress(self)
539: 
540:     def _agentCrash( self, agentIndex, quiet=False):
541:         "Helper method for handling agent crashes"
542:         if not quiet: traceback.print_exc()
543:         self.gameOver = True
544:         self.agentCrashed = True
545:         self.rules.agentCrash(self, agentIndex)
546: 
547:     OLD_STDOUT = None
548:     OLD_STDERR = None
549: 
550:     def mute(self, agentIndex):
551:         if not self.muteAgents: return
552:         global OLD_STDOUT, OLD_STDERR
553:         import cStringIO
554:         OLD_STDOUT = sys.stdout
555:         OLD_STDERR = sys.stderr
556:         sys.stdout = self.agentOutput[agentIndex]
557:         sys.stderr = self.agentOutput[agentIndex]
558: 
559:     def unmute(self):
560:         if not self.muteAgents: return
561:         global OLD_STDOUT, OLD_STDERR
562:         # Revert stdout/stderr to originals
563:         sys.stdout = OLD_STDOUT
564:         sys.stderr = OLD_STDERR
565: 
566: 
567:     def run( self ):
568:         """
569:         Main control loop for game play.
570:         """
571:         self.display.initialize(self.state.data)
572:         self.numMoves = 0
573: 
574:         ###self.display.initialize(self.state.makeObservation(1).data)
575:         # inform learning agents of the game start
576:         for i in range(len(self.agents)):
577:             agent = self.agents[i]
578:             if not agent:
579:                 self.mute(i)
580:                 # this is a null agent, meaning it failed to load
581:                 # the other team wins
582:                 print >>sys.stderr, "Agent %d failed to load" % i
583:                 self.unmute()
584:                 self._agentCrash(i, quiet=True)
585:                 return
586:             if ("registerInitialState" in dir(agent)):
587:                 self.mute(i)
588:                 if self.catchExceptions:
589:                     try:
590:                         timed_func = TimeoutFunction(agent.registerInitialState, int(self.rules.getMaxStartupTime(i)))
591:                         try:
592:                             start_time = time.time()
593:                             timed_func(self.state.deepCopy())
594:                             time_taken = time.time() - start_time
595:                             self.totalAgentTimes[i] += time_taken
596:                         except TimeoutFunctionException:
597:                             print >>sys.stderr, "Agent %d ran out of time on startup!" % i
598:                             self.unmute()
599:                             self.agentTimeout = True
600:                             self._agentCrash(i, quiet=True)
601:                             return
602:                     except Exception,data:
603:                         self._agentCrash(i, quiet=False)
604:                         self.unmute()
605:                         return
606:                 else:
607:                     agent.registerInitialState(self.state.deepCopy())
608:                 ## TODO: could this exceed the total time
609:                 self.unmute()
610: 
611:         agentIndex = self.startingIndex
612:         numAgents = len( self.agents )
613: 
614:         while not self.gameOver:
615:             # Fetch the next agent
616:             agent = self.agents[agentIndex]
617:             move_time = 0
618:             skip_action = False
619:             # Generate an observation of the state
620:             if 'observationFunction' in dir( agent ):
621:                 self.mute(agentIndex)
622:                 if self.catchExceptions:
623:                     try:
624:                         timed_func = TimeoutFunction(agent.observationFunction, int(self.rules.getMoveTimeout(agentIndex)))
625:                         try:
626:                             start_time = time.time()
627:                             observation = timed_func(self.state.deepCopy())
628:                         except TimeoutFunctionException:
629:                             skip_action = True
630:                         move_time += time.time() - start_time
631:                         self.unmute()
632:                     except Exception,data:
633:                         self._agentCrash(agentIndex, quiet=False)
634:                         self.unmute()
635:                         return
636:                 else:
637:                     observation = agent.observationFunction(self.state.deepCopy())
638:                 self.unmute()
639:             else:
640:                 observation = self.state.deepCopy()
641: 
642:             # Solicit an action
643:             action = None
644:             self.mute(agentIndex)
645:             if self.catchExceptions:
646:                 try:
647:                     timed_func = TimeoutFunction(agent.getAction, int(self.rules.getMoveTimeout(agentIndex)) - int(move_time))
648:                     try:
649:                         start_time = time.time()
650:                         if skip_action:
651:                             raise TimeoutFunctionException()
652:                         action = timed_func( observation )
653:                     except TimeoutFunctionException:
654:                         print >>sys.stderr, "Agent %d timed out on a single move!" % agentIndex
655:                         self.agentTimeout = True
656:                         self._agentCrash(agentIndex, quiet=True)
657:                         self.unmute()
658:                         return
659: 
660:                     move_time += time.time() - start_time
661: 
662:                     if move_time > self.rules.getMoveWarningTime(agentIndex):
663:                         self.totalAgentTimeWarnings[agentIndex] += 1
664:                         print >>sys.stderr, "Agent %d took too long to make a move! This is warning %d" % (agentIndex, self.totalAgentTimeWarnings[agentIndex])
665:                         if self.totalAgentTimeWarnings[agentIndex] > self.rules.getMaxTimeWarnings(agentIndex):
666:                             print >>sys.stderr, "Agent %d exceeded the maximum number of warnings: %d" % (agentIndex, self.totalAgentTimeWarnings[agentIndex])
667:                             self.agentTimeout = True
668:                             self._agentCrash(agentIndex, quiet=True)
669:                             self.unmute()
670:                             return
671: 
672:                     self.totalAgentTimes[agentIndex] += move_time
673:                     #print "Agent: %d, time: %f, total: %f" % (agentIndex, move_time, self.totalAgentTimes[agentIndex])
674:                     if self.totalAgentTimes[agentIndex] > self.rules.getMaxTotalTime(agentIndex):
675:                         print >>sys.stderr, "Agent %d ran out of time! (time: %1.2f)" % (agentIndex, self.totalAgentTimes[agentIndex])
676:                         self.agentTimeout = True
677:                         self._agentCrash(agentIndex, quiet=True)
678:                         self.unmute()
679:                         return
680:                     self.unmute()
681:                 except Exception,data:
682:                     self._agentCrash(agentIndex)
683:                     self.unmute()
684:                     return
685:             else:
686:                 action = agent.getAction(observation)
687:             self.unmute()
688: 
689:             # Execute the action
690:             self.moveHistory.append( (agentIndex, action) )
691:             if self.catchExceptions:
692:                 try:
693:                     self.state = self.state.generateSuccessor( agentIndex, action )
694:                 except Exception,data:
695:                     self.mute(agentIndex)
696:                     self._agentCrash(agentIndex)
697:                     self.unmute()
698:                     return
699:             else:
700:                 self.state = self.state.generateSuccessor( agentIndex, action )
701: 
702:             # Change the display
703:             self.display.update( self.state.data )
704:             ###idx = agentIndex - agentIndex % 2 + 1
705:             ###self.display.update( self.state.makeObservation(idx).data )
706: 
707:             # Allow for game specific conditions (winning, losing, etc.)
708:             self.rules.process(self.state, self)
709:             # Track progress
710:             if agentIndex == numAgents + 1: self.numMoves += 1
711:             # Next agent
712:             agentIndex = ( agentIndex + 1 ) % numAgents
713: 
714:             if _BOINC_ENABLED:
715:                 boinc.set_fraction_done(self.getProgress())
716: 
717:         # inform a learning agent of the game result
718:         for agentIndex, agent in enumerate(self.agents):
719:             if "final" in dir( agent ) :
720:                 try:
721:                     self.mute(agentIndex)
722:                     agent.final( self.state )
723:                     self.unmute()
724:                 except Exception,data:
725:                     if not self.catchExceptions: raise
726:                     self._agentCrash(agentIndex)
727:                     self.unmute()
728:                     return
729:         self.display.finish()
730: # keyboardAgents.py
731: # -----------------
732: # Licensing Information:  You are free to use or extend these projects for
733: # educational purposes provided that (1) you do not distribute or publish
734: # solutions, (2) you retain this notice, and (3) you provide clear
735: # attribution to UC Berkeley, including a link to http://ai.berkeley.edu.
736: # 
737: # Attribution Information: The Pacman AI projects were developed at UC Berkeley.
738: # The core projects and autograders were primarily created by John DeNero
739: # (denero@cs.berkeley.edu) and Dan Klein (klein@cs.berkeley.edu).
740: # Student side autograding was added by Brad Miller, Nick Hay, and
741: # Pieter Abbeel (pabbeel@cs.berkeley.edu).
742: 
743: 
744: from game import Agent
745: from game import Directions
746: import random
747: 
748: class KeyboardAgent(Agent):
749:     """
750:     An agent controlled by the keyboard.
751:     """
752:     # NOTE: Arrow keys also work.
753:     WEST_KEY  = 'a'
754:     EAST_KEY  = 'd'
755:     NORTH_KEY = 'w'
756:     SOUTH_KEY = 's'
757:     STOP_KEY = 'q'
758: 
759:     def __init__( self, index = 0 ):
760: 
761:         self.lastMove = Directions.STOP
762:         self.index = index
763:         self.keys = []
764: 
765:     def getAction( self, state):
766:         from graphicsUtils import keys_waiting
767:         from graphicsUtils import keys_pressed
768:         keys = keys_waiting() + keys_pressed()
769:         if keys != []:
770:             self.keys = keys
771: 
772:         legal = state.getLegalActions(self.index)
773:         move = self.getMove(legal)
774: 
775:         if move == Directions.STOP:
776:             # Try to move in the same direction as before
777:             if self.lastMove in legal:
778:                 move = self.lastMove
779: 
780:         if (self.STOP_KEY in self.keys) and Directions.STOP in legal: move = Directions.STOP
781: 
782:         if move not in legal:
783:             move = random.choice(legal)
784: 
785:         self.lastMove = move
786:         return move
787: 
788:     def getMove(self, legal):
789:         move = Directions.STOP
790:         if   (self.WEST_KEY in self.keys or 'Left' in self.keys) and Directions.WEST in legal:  move = Directions.WEST
791:         if   (self.EAST_KEY in self.keys or 'Right' in self.keys) and Directions.EAST in legal: move = Directions.EAST
792:         if   (self.NORTH_KEY in self.keys or 'Up' in self.keys) and Directions.NORTH in legal:   move = Directions.NORTH
793:         if   (self.SOUTH_KEY in self.keys or 'Down' in self.keys) and Directions.SOUTH in legal: move = Directions.SOUTH
794:         return move
795: 
796: class KeyboardAgent2(KeyboardAgent):
797:     """
798:     A second agent controlled by the keyboard.
799:     """
800:     # NOTE: Arrow keys also work.
801:     WEST_KEY  = 'j'
802:     EAST_KEY  = "l"
803:     NORTH_KEY = 'i'
804:     SOUTH_KEY = 'k'
805:     STOP_KEY = 'u'
806: 
807:     def getMove(self, legal):
808:         move = Directions.STOP
809:         if   (self.WEST_KEY in self.keys) and Directions.WEST in legal:  move = Directions.WEST
810:         if   (self.EAST_KEY in self.keys) and Directions.EAST in legal: move = Directions.EAST
811:         if   (self.NORTH_KEY in self.keys) and Directions.NORTH in legal:   move = Directions.NORTH
812:         if   (self.SOUTH_KEY in self.keys) and Directions.SOUTH in legal: move = Directions.SOUTH
813:         return move
814: # keyboardAgents.py
815: # -----------------
816: # Licensing Information:  You are free to use or extend these projects for
817: # educational purposes provided that (1) you do not distribute or publish
818: # solutions, (2) you retain this notice, and (3) you provide clear
819: # attribution to UC Berkeley, including a link to http://ai.berkeley.edu.
820: # 
821: # Attribution Information: The Pacman AI projects were developed at UC Berkeley.
822: # The core projects and autograders were primarily created by John DeNero
823: # (denero@cs.berkeley.edu) and Dan Klein (klein@cs.berkeley.edu).
824: # Student side autograding was added by Brad Miller, Nick Hay, and
825: # Pieter Abbeel (pabbeel@cs.berkeley.edu).
826: 
827: 
828: from game import Agent
829: from game import Directions
830: import random
831: 
832: class KeyboardAgent(Agent):
833:     """
834:     An agent controlled by the keyboard.
835:     """
836:     # NOTE: Arrow keys also work.
837:     WEST_KEY  = 'a'
838:     EAST_KEY  = 'd'
839:     NORTH_KEY = 'w'
840:     SOUTH_KEY = 's'
841:     STOP_KEY = 'q'
842: 
843:     def __init__( self, index = 0 ):
844: 
845:         self.lastMove = Directions.STOP
846:         self.index = index
847:         self.keys = []
848: 
849:     def getAction( self, state):
850:         from graphicsUtils import keys_waiting
851:         from graphicsUtils import keys_pressed
852:         keys = keys_waiting() + keys_pressed()
853:         if keys != []:
854:             self.keys = keys
855: 
856:         legal = state.getLegalActions(self.index)
857:         move = self.getMove(legal)
858: 
859:         if move == Directions.STOP:
860:             # Try to move in the same direction as before
861:             if self.lastMove in legal:
862:                 move = self.lastMove
863: 
864:         if (self.STOP_KEY in self.keys) and Directions.STOP in legal: move = Directions.STOP
865: 
866:         if move not in legal:
867:             move = random.choice(legal)
868: 
869:         self.lastMove = move
870:         return move
871: 
872:     def getMove(self, legal):
873:         move = Directions.STOP
874:         if   (self.WEST_KEY in self.keys or 'Left' in self.keys) and Directions.WEST in legal:  move = Directions.WEST
875:         if   (self.EAST_KEY in self.keys or 'Right' in self.keys) and Directions.EAST in legal: move = Directions.EAST
876:         if   (self.NORTH_KEY in self.keys or 'Up' in self.keys) and Directions.NORTH in legal:   move = Directions.NORTH
877:         if   (self.SOUTH_KEY in self.keys or 'Down' in self.keys) and Directions.SOUTH in legal: move = Directions.SOUTH
878:         return move
879: 
880: class KeyboardAgent2(KeyboardAgent):
881:     """
882:     A second agent controlled by the keyboard.
883:     """
884:     # NOTE: Arrow keys also work.
885:     WEST_KEY  = 'j'
886:     EAST_KEY  = "l"
887:     NORTH_KEY = 'i'
888:     SOUTH_KEY = 'k'
889:     STOP_KEY = 'u'
890: 
891:     def getMove(self, legal):
892:         move = Directions.STOP
893:         if   (self.WEST_KEY in self.keys) and Directions.WEST in legal:  move = Directions.WEST
894:         if   (self.EAST_KEY in self.keys) and Directions.EAST in legal: move = Directions.EAST
895:         if   (self.NORTH_KEY in self.keys) and Directions.NORTH in legal:   move = Directions.NORTH
896:         if   (self.SOUTH_KEY in self.keys) and Directions.SOUTH in legal: move = Directions.SOUTH
897:         return move
898: # pacman.py
899: # ---------
900: # Licensing Information:  You are free to use or extend these projects for
901: # educational purposes provided that (1) you do not distribute or publish
902: # solutions, (2) you retain this notice, and (3) you provide clear
903: # attribution to UC Berkeley, including a link to http://ai.berkeley.edu.
904: # 
905: # Attribution Information: The Pacman AI projects were developed at UC Berkeley.
906: # The core projects and autograders were primarily created by John DeNero
907: # (denero@cs.berkeley.edu) and Dan Klein (klein@cs.berkeley.edu).
908: # Student side autograding was added by Brad Miller, Nick Hay, and
909: # Pieter Abbeel (pabbeel@cs.berkeley.edu).
910: 
911: 
912: """
913: Pacman.py holds the logic for the classic pacman game along with the main
914: code to run a game.  This file is divided into three sections:
915: 
916:   (i)  Your interface to the pacman world:
917:           Pacman is a complex environment.  You probably don't want to
918:           read through all of the code we wrote to make the game runs
919:           correctly.  This section contains the parts of the code
920:           that you will need to understand in order to complete the
921:           project.  There is also some code in game.py that you should
922:           understand.
923: 
924:   (ii)  The hidden secrets of pacman:
925:           This section contains all of the logic code that the pacman
926:           environment uses to decide who can move where, who dies when
927:           things collide, etc.  You shouldn't need to read this section
928:           of code, but you can if you want.
929: 
930:   (iii) Framework to start a game:
931:           The final section contains the code for reading the command
932:           you use to set up the game, then starting up a new game, along with
933:           linking in all the external parts (agent functions, graphics).
934:           Check this section out to see all the options available to you.
935: 
936: To play your first game, type 'python pacman.py' from the command line.
937: The keys are 'a', 's', 'd', and 'w' to move (or arrow keys).  Have fun!
938: """
939: from game import GameStateData
940: from game import Game
941: from game import Directions
942: from game import Actions
943: from util import nearestPoint
944: from util import manhattanDistance
945: import util, layout
946: import sys, types, time, random, os
947: 
948: ###################################################
949: # YOUR INTERFACE TO THE PACMAN WORLD: A GameState #
950: ###################################################
951: 
952: class GameState:
953:     """
954:     A GameState specifies the full game state, including the food, capsules,
955:     agent configurations and score changes.
956: 
957:     GameStates are used by the Game object to capture the actual state of the game and
958:     can be used by agents to reason about the game.
959: 
960:     Much of the information in a GameState is stored in a GameStateData object.  We
961:     strongly suggest that you access that data via the accessor methods below rather
962:     than referring to the GameStateData object directly.
963: 
964:     Note that in classic Pacman, Pacman is always agent 0.
965:     """
966: 
967:     ####################################################
968:     # Accessor methods: use these to access state data #
969:     ####################################################
970: 
971:     # static variable keeps track of which states have had getLegalActions called
972:     explored = set()
973:     def getAndResetExplored():
974:         tmp = GameState.explored.copy()
975:         GameState.explored = set()
976:         return tmp
977:     getAndResetExplored = staticmethod(getAndResetExplored)
978: 
979:     def getLegalActions( self, agentIndex=0 ):
980:         """
981:         Returns the legal actions for the agent specified.
982:         """
983: #        GameState.explored.add(self)
984:         if self.isWin() or self.isLose(): return []
985: 
986:         if agentIndex == 0:  # Pacman is moving
987:             return PacmanRules.getLegalActions( self )
988:         else:
989:             return GhostRules.getLegalActions( self, agentIndex )
990: 
991:     def generateSuccessor( self, agentIndex, action):
992:         """
993:         Returns the successor state after the specified agent takes the action.
994:         """
995:         # Check that successors exist
996:         if self.isWin() or self.isLose(): raise Exception('Can\'t generate a successor of a terminal state.')
997: 
998:         # Copy current state
999:         state = GameState(self)
1000: 
1001:         # Let agent's logic deal with its action's effects on the board
1002:         if agentIndex == 0:  # Pacman is moving
1003:             state.data._eaten = [False for i in range(state.getNumAgents())]
1004:             PacmanRules.applyAction( state, action )
1005:         else:                # A ghost is moving
1006:             GhostRules.applyAction( state, action, agentIndex )
1007: 
1008:         # Time passes
1009:         if agentIndex == 0:
1010:             state.data.scoreChange += -TIME_PENALTY # Penalty for waiting around
1011:         else:
1012:             GhostRules.decrementTimer( state.data.agentStates[agentIndex] )
1013: 
1014:         # Resolve multi-agent effects
1015:         GhostRules.checkDeath( state, agentIndex )
1016: 
1017:         # Book keeping
1018:         state.data._agentMoved = agentIndex
1019:         state.data.score += state.data.scoreChange
1020:         GameState.explored.add(self)
1021:         GameState.explored.add(state)
1022:         return state
1023: 
1024:     def getLegalPacmanActions( self ):
1025:         return self.getLegalActions( 0 )
1026: 
1027:     def generatePacmanSuccessor( self, action ):
1028:         """
1029:         Generates the successor state after the specified pacman move
1030:         """
1031:         return self.generateSuccessor( 0, action )
1032: 
1033:     def getPacmanState( self ):
1034:         """
1035:         Returns an AgentState object for pacman (in game.py)
1036: 
1037:         state.pos gives the current position
1038:         state.direction gives the travel vector
1039:         """
1040:         return self.data.agentStates[0].copy()
1041: 
1042:     def getPacmanPosition( self ):
1043:         return self.data.agentStates[0].getPosition()
1044: 
1045:     def getGhostStates( self ):
1046:         return self.data.agentStates[1:]
1047: 
1048:     def getGhostState( self, agentIndex ):
1049:         if agentIndex == 0 or agentIndex >= self.getNumAgents():
1050:             raise Exception("Invalid index passed to getGhostState")
1051:         return self.data.agentStates[agentIndex]
1052: 
1053:     def getGhostPosition( self, agentIndex ):
1054:         if agentIndex == 0:
1055:             raise Exception("Pacman's index passed to getGhostPosition")
1056:         return self.data.agentStates[agentIndex].getPosition()
1057: 
1058:     def getGhostPositions(self):
1059:         return [s.getPosition() for s in self.getGhostStates()]
1060: 
1061:     def getNumAgents( self ):
1062:         return len( self.data.agentStates )
1063: 
1064:     def getScore( self ):
1065:         return float(self.data.score)
1066: 
1067:     def getCapsules(self):
1068:         """
1069:         Returns a list of positions (x,y) of the remaining capsules.
1070:         """
1071:         return self.data.capsules
1072: 
1073:     def getNumFood( self ):
1074:         return self.data.food.count()
1075: 
1076:     def getFood(self):
1077:         """
1078:         Returns a Grid of boolean food indicator variables.
1079: 
1080:         Grids can be accessed via list notation, so to check
1081:         if there is food at (x,y), just call
1082: 
1083:         currentFood = state.getFood()
1084:         if currentFood[x][y] == True: ...
1085:         """
1086:         return self.data.food
1087: 
1088:     def getWalls(self):
1089:         """
1090:         Returns a Grid of boolean wall indicator variables.
1091: 
1092:         Grids can be accessed via list notation, so to check
1093:         if there is a wall at (x,y), just call
1094: 
1095:         walls = state.getWalls()
1096:         if walls[x][y] == True: ...
1097:         """
1098:         return self.data.layout.walls
1099: 
1100:     def hasFood(self, x, y):
1101:         return self.data.food[x][y]
1102: 
1103:     def hasWall(self, x, y):
1104:         return self.data.layout.walls[x][y]
1105: 
1106:     def isLose( self ):
1107:         return self.data._lose
1108: 
1109:     def isWin( self ):
1110:         return self.data._win
1111: 
1112:     #############################################
1113:     #             Helper methods:               #
1114:     # You shouldn't need to call these directly #
1115:     #############################################
1116: 
1117:     def __init__( self, prevState = None ):
1118:         """
1119:         Generates a new state by copying information from its predecessor.
1120:         """
1121:         if prevState != None: # Initial state
1122:             self.data = GameStateData(prevState.data)
1123:         else:
1124:             self.data = GameStateData()
1125: 
1126:     def deepCopy( self ):
1127:         state = GameState( self )
1128:         state.data = self.data.deepCopy()
1129:         return state
1130: 
1131:     def __eq__( self, other ):
1132:         """
1133:         Allows two states to be compared.
1134:         """
1135:         return hasattr(other, 'data') and self.data == other.data
1136: 
1137:     def __hash__( self ):
1138:         """
1139:         Allows states to be keys of dictionaries.
1140:         """
1141:         return hash( self.data )
1142: 
1143:     def __str__( self ):
1144: 
1145:         return str(self.data)
1146: 
1147:     def initialize( self, layout, numGhostAgents=1000 ):
1148:         """
1149:         Creates an initial game state from a layout array (see layout.py).
1150:         """
1151:         self.data.initialize(layout, numGhostAgents)
1152: 
1153: ############################################################################
1154: #                     THE HIDDEN SECRETS OF PACMAN                         #
1155: #                                                                          #
1156: # You shouldn't need to look through the code in this section of the file. #
1157: ############################################################################
1158: 
1159: SCARED_TIME = 40    # Moves ghosts are scared
1160: COLLISION_TOLERANCE = 0.7 # How close ghosts must be to Pacman to kill
1161: TIME_PENALTY = 1 # Number of points lost each round
1162: 
1163: class ClassicGameRules:
1164:     """
1165:     These game rules manage the control flow of a game, deciding when
1166:     and how the game starts and ends.
1167:     """
1168:     def __init__(self, timeout=30):
1169:         self.timeout = timeout
1170: 
1171:     def newGame( self, layout, pacmanAgent, ghostAgents, display, quiet = False, catchExceptions=False):
1172:         agents = [pacmanAgent] + ghostAgents[:layout.getNumGhosts()]
1173:         initState = GameState()
1174:         initState.initialize( layout, len(ghostAgents) )
1175:         game = Game(agents, display, self, catchExceptions=catchExceptions)
1176:         game.state = initState
1177:         self.initialState = initState.deepCopy()
1178:         self.quiet = quiet
1179:         return game
1180: 
1181:     def process(self, state, game):
1182:         """
1183:         Checks to see whether it is time to end the game.
1184:         """
1185:         if state.isWin(): self.win(state, game)
1186:         if state.isLose(): self.lose(state, game)
1187: 
1188:     def win( self, state, game ):
1189:         if not self.quiet: print "Pacman emerges victorious! Score: %d" % state.data.score
1190:         game.gameOver = True
1191: 
1192:     def lose( self, state, game ):
1193:         if not self.quiet: print "Pacman died! Score: %d" % state.data.score
1194:         game.gameOver = True
1195: 
1196:     def getProgress(self, game):
1197:         return float(game.state.getNumFood()) / self.initialState.getNumFood()
1198: 
1199:     def agentCrash(self, game, agentIndex):
1200:         if agentIndex == 0:
1201:             print "Pacman crashed"
1202:         else:
1203:             print "A ghost crashed"
1204: 
1205:     def getMaxTotalTime(self, agentIndex):
1206:         return self.timeout
1207: 
1208:     def getMaxStartupTime(self, agentIndex):
1209:         return self.timeout
1210: 
1211:     def getMoveWarningTime(self, agentIndex):
1212:         return self.timeout
1213: 
1214:     def getMoveTimeout(self, agentIndex):
1215:         return self.timeout
1216: 
1217:     def getMaxTimeWarnings(self, agentIndex):
1218:         return 0
1219: 
1220: class PacmanRules:
1221:     """
1222:     These functions govern how pacman interacts with his environment under
1223:     the classic game rules.
1224:     """
1225:     PACMAN_SPEED=1
1226: 
1227:     def getLegalActions( state ):
1228:         """
1229:         Returns a list of possible actions.
1230:         """
1231:         return Actions.getPossibleActions( state.getPacmanState().configuration, state.data.layout.walls )
1232:     getLegalActions = staticmethod( getLegalActions )
1233: 
1234:     def applyAction( state, action ):
1235:         """
1236:         Edits the state to reflect the results of the action.
1237:         """
1238:         legal = PacmanRules.getLegalActions( state )
1239:         if action not in legal:
1240:             raise Exception("Illegal action " + str(action))
1241: 
1242:         pacmanState = state.data.agentStates[0]
1243: 
1244:         # Update Configuration
1245:         vector = Actions.directionToVector( action, PacmanRules.PACMAN_SPEED )
1246:         pacmanState.configuration = pacmanState.configuration.generateSuccessor( vector )
1247: 
1248:         # Eat
1249:         next = pacmanState.configuration.getPosition()
1250:         nearest = nearestPoint( next )
1251:         if manhattanDistance( nearest, next ) <= 0.5 :
1252:             # Remove food
1253:             PacmanRules.consume( nearest, state )
1254:     applyAction = staticmethod( applyAction )
1255: 
1256:     def consume( position, state ):
1257:         x,y = position
1258:         # Eat food
1259:         if state.data.food[x][y]:
1260:             state.data.scoreChange += 10
1261:             state.data.food = state.data.food.copy()
1262:             state.data.food[x][y] = False
1263:             state.data._foodEaten = position
1264:             # TODO: cache numFood?
1265:             numFood = state.getNumFood()
1266:             if numFood == 0 and not state.data._lose:
1267:                 state.data.scoreChange += 500
1268:                 state.data._win = True
1269:         # Eat capsule
1270:         if( position in state.getCapsules() ):
1271:             state.data.capsules.remove( position )
1272:             state.data._capsuleEaten = position
1273:             # Reset all ghosts' scared timers
1274:             for index in range( 1, len( state.data.agentStates ) ):
1275:                 state.data.agentStates[index].scaredTimer = SCARED_TIME
1276:     consume = staticmethod( consume )
1277: 
1278: class GhostRules:
1279:     """
1280:     These functions dictate how ghosts interact with their environment.
1281:     """
1282:     GHOST_SPEED=1.0
1283:     def getLegalActions( state, ghostIndex ):
1284:         """
1285:         Ghosts cannot stop, and cannot turn around unless they
1286:         reach a dead end, but can turn 90 degrees at intersections.
1287:         """
1288:         conf = state.getGhostState( ghostIndex ).configuration
1289:         possibleActions = Actions.getPossibleActions( conf, state.data.layout.walls )
1290:         reverse = Actions.reverseDirection( conf.direction )
1291:         if Directions.STOP in possibleActions:
1292:             possibleActions.remove( Directions.STOP )
1293:         if reverse in possibleActions and len( possibleActions ) > 1:
1294:             possibleActions.remove( reverse )
1295:         return possibleActions
1296:     getLegalActions = staticmethod( getLegalActions )
1297: 
1298:     def applyAction( state, action, ghostIndex):
1299: 
1300:         legal = GhostRules.getLegalActions( state, ghostIndex )
1301:         if action not in legal:
1302:             raise Exception("Illegal ghost action " + str(action))
1303: 
1304:         ghostState = state.data.agentStates[ghostIndex]
1305:         speed = GhostRules.GHOST_SPEED
1306:         if ghostState.scaredTimer > 0: speed /= 2.0
1307:         vector = Actions.directionToVector( action, speed )
1308:         ghostState.configuration = ghostState.configuration.generateSuccessor( vector )
1309:     applyAction = staticmethod( applyAction )
1310: 
1311:     def decrementTimer( ghostState):
1312:         timer = ghostState.scaredTimer
1313:         if timer == 1:
1314:             ghostState.configuration.pos = nearestPoint( ghostState.configuration.pos )
1315:         ghostState.scaredTimer = max( 0, timer - 1 )
1316:     decrementTimer = staticmethod( decrementTimer )
1317: 
1318:     def checkDeath( state, agentIndex):
1319:         pacmanPosition = state.getPacmanPosition()
1320:         if agentIndex == 0: # Pacman just moved; Anyone can kill him
1321:             for index in range( 1, len( state.data.agentStates ) ):
1322:                 ghostState = state.data.agentStates[index]
1323:                 ghostPosition = ghostState.configuration.getPosition()
1324:                 if GhostRules.canKill( pacmanPosition, ghostPosition ):
1325:                     GhostRules.collide( state, ghostState, index )
1326:         else:
1327:             ghostState = state.data.agentStates[agentIndex]
1328:             ghostPosition = ghostState.configuration.getPosition()
1329:             if GhostRules.canKill( pacmanPosition, ghostPosition ):
1330:                 GhostRules.collide( state, ghostState, agentIndex )
1331:     checkDeath = staticmethod( checkDeath )
1332: 
1333:     def collide( state, ghostState, agentIndex):
1334:         if ghostState.scaredTimer > 0:
1335:             state.data.scoreChange += 200
1336:             GhostRules.placeGhost(state, ghostState)
1337:             ghostState.scaredTimer = 0
1338:             # Added for first-person
1339:             state.data._eaten[agentIndex] = True
1340:         else:
1341:             if not state.data._win:
1342:                 state.data.scoreChange -= 500
1343:                 state.data._lose = True
1344:     collide = staticmethod( collide )
1345: 
1346:     def canKill( pacmanPosition, ghostPosition ):
1347:         return manhattanDistance( ghostPosition, pacmanPosition ) <= COLLISION_TOLERANCE
1348:     canKill = staticmethod( canKill )
1349: 
1350:     def placeGhost(state, ghostState):
1351:         ghostState.configuration = ghostState.start
1352:     placeGhost = staticmethod( placeGhost )
1353: 
1354: #############################
1355: # FRAMEWORK TO START A GAME #
1356: #############################
1357: 
1358: def default(str):
1359:     return str + ' [Default: %default]'
1360: 
1361: def parseAgentArgs(str):
1362:     if str == None: return {}
1363:     pieces = str.split(',')
1364:     opts = {}
1365:     for p in pieces:
1366:         if '=' in p:
1367:             key, val = p.split('=')
1368:         else:
1369:             key,val = p, 1
1370:         opts[key] = val
1371:     return opts
1372: 
1373: def readCommand( argv ):
1374:     """
1375:     Processes the command used to run pacman from the command line.
1376:     """
1377:     from optparse import OptionParser
1378:     usageStr = """
1379:     USAGE:      python pacman.py <options>
1380:     EXAMPLES:   (1) python pacman.py
1381:                     - starts an interactive game
1382:                 (2) python pacman.py --layout smallClassic --zoom 2
1383:                 OR  python pacman.py -l smallClassic -z 2
1384:                     - starts an interactive game on a smaller board, zoomed in
1385:     """
1386:     parser = OptionParser(usageStr)
1387: 
1388:     parser.add_option('-n', '--numGames', dest='numGames', type='int',
1389:                       help=default('the number of GAMES to play'), metavar='GAMES', default=1)
1390:     parser.add_option('-l', '--layout', dest='layout',
1391:                       help=default('the LAYOUT_FILE from which to load the map layout'),
1392:                       metavar='LAYOUT_FILE', default='mediumClassic')
1393:     parser.add_option('-p', '--pacman', dest='pacman',
1394:                       help=default('the agent TYPE in the pacmanAgents module to use'),
1395:                       metavar='TYPE', default='KeyboardAgent')
1396:     parser.add_option('-t', '--textGraphics', action='store_true', dest='textGraphics',
1397:                       help='Display output as text only', default=False)
1398:     parser.add_option('-q', '--quietTextGraphics', action='store_true', dest='quietGraphics',
1399:                       help='Generate minimal output and no graphics', default=False)
1400:     parser.add_option('-g', '--ghosts', dest='ghost',
1401:                       help=default('the ghost agent TYPE in the ghostAgents module to use'),
1402:                       metavar = 'TYPE', default='RandomGhost')
1403:     parser.add_option('-k', '--numghosts', type='int', dest='numGhosts',
1404:                       help=default('The maximum number of ghosts to use'), default=4)
1405:     parser.add_option('-z', '--zoom', type='float', dest='zoom',
1406:                       help=default('Zoom the size of the graphics window'), default=1.0)
1407:     parser.add_option('-f', '--fixRandomSeed', action='store_true', dest='fixRandomSeed',
1408:                       help='Fixes the random seed to always play the same game', default=False)
1409:     parser.add_option('-r', '--recordActions', action='store_true', dest='record',
1410:                       help='Writes game histories to a file (named by the time they were played)', default=False)
1411:     parser.add_option('--replay', dest='gameToReplay',
1412:                       help='A recorded game file (pickle) to replay', default=None)
1413:     parser.add_option('-a','--agentArgs',dest='agentArgs',
1414:                       help='Comma separated values sent to agent. e.g. "opt1=val1,opt2,opt3=val3"')
1415:     parser.add_option('-x', '--numTraining', dest='numTraining', type='int',
1416:                       help=default('How many episodes are training (suppresses output)'), default=0)
1417:     parser.add_option('--frameTime', dest='frameTime', type='float',
1418:                       help=default('Time to delay between frames; <0 means keyboard'), default=0.1)
1419:     parser.add_option('-c', '--catchExceptions', action='store_true', dest='catchExceptions',
1420:                       help='Turns on exception handling and timeouts during games', default=False)
1421:     parser.add_option('--timeout', dest='timeout', type='int',
1422:                       help=default('Maximum length of time an agent can spend computing in a single game'), default=30)
1423: 
1424:     options, otherjunk = parser.parse_args(argv)
1425:     if len(otherjunk) != 0:
1426:         raise Exception('Command line input not understood: ' + str(otherjunk))
1427:     args = dict()
1428: 
1429:     # Fix the random seed
1430:     if options.fixRandomSeed: random.seed('cs188')
1431: 
1432:     # Choose a layout
1433:     args['layout'] = layout.getLayout( options.layout )
1434:     if args['layout'] == None: raise Exception("The layout " + options.layout + " cannot be found")
1435: 
1436:     # Choose a Pacman agent
1437:     noKeyboard = options.gameToReplay == None and (options.textGraphics or options.quietGraphics)
1438:     pacmanType = loadAgent(options.pacman, noKeyboard)
1439:     agentOpts = parseAgentArgs(options.agentArgs)
1440:     if options.numTraining > 0:
1441:         args['numTraining'] = options.numTraining
1442:         if 'numTraining' not in agentOpts: agentOpts['numTraining'] = options.numTraining
1443:     pacman = pacmanType(**agentOpts) # Instantiate Pacman with agentArgs
1444:     args['pacman'] = pacman
1445: 
1446:     # Don't display training games
1447:     if 'numTrain' in agentOpts:
1448:         options.numQuiet = int(agentOpts['numTrain'])
1449:         options.numIgnore = int(agentOpts['numTrain'])
1450: 
1451:     # Choose a ghost agent
1452:     ghostType = loadAgent(options.ghost, noKeyboard)
1453:     args['ghosts'] = [ghostType( i+1 ) for i in range( options.numGhosts )]
1454: 
1455:     # Choose a display format
1456:     if options.quietGraphics:
1457:         import textDisplay
1458:         args['display'] = textDisplay.NullGraphics()
1459:     elif options.textGraphics:
1460:         import textDisplay
1461:         textDisplay.SLEEP_TIME = options.frameTime
1462:         args['display'] = textDisplay.PacmanGraphics()
1463:     else:
1464:         import graphicsDisplay
1465:         args['display'] = graphicsDisplay.PacmanGraphics(options.zoom, frameTime = options.frameTime)
1466:     args['numGames'] = options.numGames
1467:     args['record'] = options.record
1468:     args['catchExceptions'] = options.catchExceptions
1469:     args['timeout'] = options.timeout
1470: 
1471:     # Special case: recorded games don't use the runGames method or args structure
1472:     if options.gameToReplay != None:
1473:         print 'Replaying recorded game %s.' % options.gameToReplay
1474:         import cPickle
1475:         f = open(options.gameToReplay)
1476:         try: recorded = cPickle.load(f)
1477:         finally: f.close()
1478:         recorded['display'] = args['display']
1479:         replayGame(**recorded)
1480:         sys.exit(0)
1481: 
1482:     return args
1483: 
1484: def loadAgent(pacman, nographics):
1485:     # Looks through all pythonPath Directories for the right module,
1486:     pythonPathStr = os.path.expandvars("$PYTHONPATH")
1487:     if pythonPathStr.find(';') == -1:
1488:         pythonPathDirs = pythonPathStr.split(':')
1489:     else:
1490:         pythonPathDirs = pythonPathStr.split(';')
1491:     pythonPathDirs.append('.')
1492: 
1493:     for moduleDir in pythonPathDirs:
1494:         if not os.path.isdir(moduleDir): continue
1495:         moduleNames = [f for f in os.listdir(moduleDir) if f.endswith('gents.py')]
1496:         for modulename in moduleNames:
1497:             try:
1498:                 module = __import__(modulename[:-3])
1499:             except ImportError:
1500:                 continue
1501:             if pacman in dir(module):
1502:                 if nographics and modulename == 'keyboardAgents.py':
1503:                     raise Exception('Using the keyboard requires graphics (not text display)')
1504:                 return getattr(module, pacman)
1505:     raise Exception('The agent ' + pacman + ' is not specified in any *Agents.py.')
1506: 
1507: def replayGame( layout, actions, display ):
1508:     import pacmanAgents, ghostAgents
1509:     rules = ClassicGameRules()
1510:     agents = [pacmanAgents.GreedyAgent()] + [ghostAgents.RandomGhost(i+1) for i in range(layout.getNumGhosts())]
1511:     game = rules.newGame( layout, agents[0], agents[1:], display )
1512:     state = game.state
1513:     display.initialize(state.data)
1514: 
1515:     for action in actions:
1516:             # Execute the action
1517:         state = state.generateSuccessor( *action )
1518:         # Change the display
1519:         display.update( state.data )
1520:         # Allow for game specific conditions (winning, losing, etc.)
1521:         rules.process(state, game)
1522: 
1523:     display.finish()
1524: 
1525: def runGames( layout, pacman, ghosts, display, numGames, record, numTraining = 0, catchExceptions=False, timeout=30 ):
1526:     import __main__
1527:     __main__.__dict__['_display'] = display
1528: 
1529:     rules = ClassicGameRules(timeout)
1530:     games = []
1531: 
1532:     for i in range( numGames ):
1533:         beQuiet = i < numTraining
1534:         if beQuiet:
1535:                 # Suppress output and graphics
1536:             import textDisplay
1537:             gameDisplay = textDisplay.NullGraphics()
1538:             rules.quiet = True
1539:         else:
1540:             gameDisplay = display
1541:             rules.quiet = False
1542:         game = rules.newGame( layout, pacman, ghosts, gameDisplay, beQuiet, catchExceptions)
1543:         game.run()
1544:         if not beQuiet: games.append(game)
1545: 
1546:         if record:
1547:             import time, cPickle
1548:             fname = ('recorded-game-%d' % (i + 1)) +  '-'.join([str(t) for t in time.localtime()[1:6]])
1549:             f = file(fname, 'w')
1550:             components = {'layout': layout, 'actions': game.moveHistory}
1551:             cPickle.dump(components, f)
1552:             f.close()
1553: 
1554:     if (numGames-numTraining) > 0:
1555:         scores = [game.state.getScore() for game in games]
1556:         wins = [game.state.isWin() for game in games]
1557:         winRate = wins.count(True)/ float(len(wins))
1558:         print 'Average Score:', sum(scores) / float(len(scores))
1559:         print 'Scores:       ', ', '.join([str(score) for score in scores])
1560:         print 'Win Rate:      %d/%d (%.2f)' % (wins.count(True), len(wins), winRate)
1561:         print 'Record:       ', ', '.join([ ['Loss', 'Win'][int(w)] for w in wins])
1562: 
1563:     return games
1564: 
1565: if __name__ == '__main__':
1566:     """
1567:     The main function called when pacman.py is run
1568:     from the command line:
1569: 
1570:     > python pacman.py
1571: 
1572:     See the usage string for more details.
1573: 
1574:     > python pacman.py --help
1575:     """
1576:     args = readCommand( sys.argv[1:] ) # Get game components based on input
1577:     runGames( **args )
1578: 
1579:     # import cProfile
1580:     # cProfile.run("runGames( **args )")
1581:     pass
1582: # util.py
1583: # -------
1584: # Licensing Information:  You are free to use or extend these projects for
1585: # educational purposes provided that (1) you do not distribute or publish
1586: # solutions, (2) you retain this notice, and (3) you provide clear
1587: # attribution to UC Berkeley, including a link to http://ai.berkeley.edu.
1588: # 
1589: # Attribution Information: The Pacman AI projects were developed at UC Berkeley.
1590: # The core projects and autograders were primarily created by John DeNero
1591: # (denero@cs.berkeley.edu) and Dan Klein (klein@cs.berkeley.edu).
1592: # Student side autograding was added by Brad Miller, Nick Hay, and
1593: # Pieter Abbeel (pabbeel@cs.berkeley.edu).
1594: 
1595: 
1596: # util.py
1597: # -------
1598: # Licensing Information:  You are free to use or extend these projects for
1599: # educational purposes provided that (1) you do not distribute or publish
1600: # solutions, (2) you retain this notice, and (3) you provide clear
1601: # attribution to UC Berkeley, including a link to http://ai.berkeley.edu.
1602: # 
1603: # Attribution Information: The Pacman AI projects were developed at UC Berkeley.
1604: # The core projects and autograders were primarily created by John DeNero
1605: # (denero@cs.berkeley.edu) and Dan Klein (klein@cs.berkeley.edu).
1606: # Student side autograding was added by Brad Miller, Nick Hay, and
1607: # Pieter Abbeel (pabbeel@cs.berkeley.edu).
1608: 
1609: 
1610: import sys
1611: import inspect
1612: import heapq, random
1613: import cStringIO
1614: 
1615: 
1616: class FixedRandom:
1617:     def __init__(self):
1618:         fixedState = (3, (2147483648L, 507801126L, 683453281L, 310439348L, 2597246090L, \
1619:             2209084787L, 2267831527L, 979920060L, 3098657677L, 37650879L, 807947081L, 3974896263L, \
1620:             881243242L, 3100634921L, 1334775171L, 3965168385L, 746264660L, 4074750168L, 500078808L, \
1621:             776561771L, 702988163L, 1636311725L, 2559226045L, 157578202L, 2498342920L, 2794591496L, \
1622:             4130598723L, 496985844L, 2944563015L, 3731321600L, 3514814613L, 3362575829L, 3038768745L, \
1623:             2206497038L, 1108748846L, 1317460727L, 3134077628L, 988312410L, 1674063516L, 746456451L, \
1624:             3958482413L, 1857117812L, 708750586L, 1583423339L, 3466495450L, 1536929345L, 1137240525L, \
1625:             3875025632L, 2466137587L, 1235845595L, 4214575620L, 3792516855L, 657994358L, 1241843248L, \
1626:             1695651859L, 3678946666L, 1929922113L, 2351044952L, 2317810202L, 2039319015L, 460787996L, \
1627:             3654096216L, 4068721415L, 1814163703L, 2904112444L, 1386111013L, 574629867L, 2654529343L, \
1628:             3833135042L, 2725328455L, 552431551L, 4006991378L, 1331562057L, 3710134542L, 303171486L, \
1629:             1203231078L, 2670768975L, 54570816L, 2679609001L, 578983064L, 1271454725L, 3230871056L, \
1630:             2496832891L, 2944938195L, 1608828728L, 367886575L, 2544708204L, 103775539L, 1912402393L, \
1631:             1098482180L, 2738577070L, 3091646463L, 1505274463L, 2079416566L, 659100352L, 839995305L, \
1632:             1696257633L, 274389836L, 3973303017L, 671127655L, 1061109122L, 517486945L, 1379749962L, \
1633:             3421383928L, 3116950429L, 2165882425L, 2346928266L, 2892678711L, 2936066049L, 1316407868L, \
1634:             2873411858L, 4279682888L, 2744351923L, 3290373816L, 1014377279L, 955200944L, 4220990860L, \
1635:             2386098930L, 1772997650L, 3757346974L, 1621616438L, 2877097197L, 442116595L, 2010480266L, \
1636:             2867861469L, 2955352695L, 605335967L, 2222936009L, 2067554933L, 4129906358L, 1519608541L, \
1637:             1195006590L, 1942991038L, 2736562236L, 279162408L, 1415982909L, 4099901426L, 1732201505L, \
1638:             2934657937L, 860563237L, 2479235483L, 3081651097L, 2244720867L, 3112631622L, 1636991639L, \
1639:             3860393305L, 2312061927L, 48780114L, 1149090394L, 2643246550L, 1764050647L, 3836789087L, \
1640:             3474859076L, 4237194338L, 1735191073L, 2150369208L, 92164394L, 756974036L, 2314453957L, \
1641:             323969533L, 4267621035L, 283649842L, 810004843L, 727855536L, 1757827251L, 3334960421L, \
1642:             3261035106L, 38417393L, 2660980472L, 1256633965L, 2184045390L, 811213141L, 2857482069L, \
1643:             2237770878L, 3891003138L, 2787806886L, 2435192790L, 2249324662L, 3507764896L, 995388363L, \
1644:             856944153L, 619213904L, 3233967826L, 3703465555L, 3286531781L, 3863193356L, 2992340714L, \
1645:             413696855L, 3865185632L, 1704163171L, 3043634452L, 2225424707L, 2199018022L, 3506117517L, \
1646:             3311559776L, 3374443561L, 1207829628L, 668793165L, 1822020716L, 2082656160L, 1160606415L, \
1647:             3034757648L, 741703672L, 3094328738L, 459332691L, 2702383376L, 1610239915L, 4162939394L, \
1648:             557861574L, 3805706338L, 3832520705L, 1248934879L, 3250424034L, 892335058L, 74323433L, \
1649:             3209751608L, 3213220797L, 3444035873L, 3743886725L, 1783837251L, 610968664L, 580745246L, \
1650:             4041979504L, 201684874L, 2673219253L, 1377283008L, 3497299167L, 2344209394L, 2304982920L, \
1651:             3081403782L, 2599256854L, 3184475235L, 3373055826L, 695186388L, 2423332338L, 222864327L, \
1652:             1258227992L, 3627871647L, 3487724980L, 4027953808L, 3053320360L, 533627073L, 3026232514L, \
1653:             2340271949L, 867277230L, 868513116L, 2158535651L, 2487822909L, 3428235761L, 3067196046L, \
1654:             3435119657L, 1908441839L, 788668797L, 3367703138L, 3317763187L, 908264443L, 2252100381L, \
1655:             764223334L, 4127108988L, 384641349L, 3377374722L, 1263833251L, 1958694944L, 3847832657L, \
1656:             1253909612L, 1096494446L, 555725445L, 2277045895L, 3340096504L, 1383318686L, 4234428127L, \
1657:             1072582179L, 94169494L, 1064509968L, 2681151917L, 2681864920L, 734708852L, 1338914021L, \
1658:             1270409500L, 1789469116L, 4191988204L, 1716329784L, 2213764829L, 3712538840L, 919910444L, \
1659:             1318414447L, 3383806712L, 3054941722L, 3378649942L, 1205735655L, 1268136494L, 2214009444L, \
1660:             2532395133L, 3232230447L, 230294038L, 342599089L, 772808141L, 4096882234L, 3146662953L, \
1661:             2784264306L, 1860954704L, 2675279609L, 2984212876L, 2466966981L, 2627986059L, 2985545332L, \
1662:             2578042598L, 1458940786L, 2944243755L, 3959506256L, 1509151382L, 325761900L, 942251521L, \
1663:             4184289782L, 2756231555L, 3297811774L, 1169708099L, 3280524138L, 3805245319L, 3227360276L, \
1664:             3199632491L, 2235795585L, 2865407118L, 36763651L, 2441503575L, 3314890374L, 1755526087L, \
1665:             17915536L, 1196948233L, 949343045L, 3815841867L, 489007833L, 2654997597L, 2834744136L, \
1666:             417688687L, 2843220846L, 85621843L, 747339336L, 2043645709L, 3520444394L, 1825470818L, \
1667:             647778910L, 275904777L, 1249389189L, 3640887431L, 4200779599L, 323384601L, 3446088641L, \
1668:             4049835786L, 1718989062L, 3563787136L, 44099190L, 3281263107L, 22910812L, 1826109246L, \
1669:             745118154L, 3392171319L, 1571490704L, 354891067L, 815955642L, 1453450421L, 940015623L, \
1670:             796817754L, 1260148619L, 3898237757L, 176670141L, 1870249326L, 3317738680L, 448918002L, \
1671:             4059166594L, 2003827551L, 987091377L, 224855998L, 3520570137L, 789522610L, 2604445123L, \
1672:             454472869L, 475688926L, 2990723466L, 523362238L, 3897608102L, 806637149L, 2642229586L, \
1673:             2928614432L, 1564415411L, 1691381054L, 3816907227L, 4082581003L, 1895544448L, 3728217394L, \
1674:             3214813157L, 4054301607L, 1882632454L, 2873728645L, 3694943071L, 1297991732L, 2101682438L, \
1675:             3952579552L, 678650400L, 1391722293L, 478833748L, 2976468591L, 158586606L, 2576499787L, \
1676:             662690848L, 3799889765L, 3328894692L, 2474578497L, 2383901391L, 1718193504L, 3003184595L, \
1677:             3630561213L, 1929441113L, 3848238627L, 1594310094L, 3040359840L, 3051803867L, 2462788790L, \
1678:             954409915L, 802581771L, 681703307L, 545982392L, 2738993819L, 8025358L, 2827719383L, \
1679:             770471093L, 3484895980L, 3111306320L, 3900000891L, 2116916652L, 397746721L, 2087689510L, \
1680:             721433935L, 1396088885L, 2751612384L, 1998988613L, 2135074843L, 2521131298L, 707009172L, \
1681:             2398321482L, 688041159L, 2264560137L, 482388305L, 207864885L, 3735036991L, 3490348331L, \
1682:             1963642811L, 3260224305L, 3493564223L, 1939428454L, 1128799656L, 1366012432L, 2858822447L, \
1683:             1428147157L, 2261125391L, 1611208390L, 1134826333L, 2374102525L, 3833625209L, 2266397263L, \
1684:             3189115077L, 770080230L, 2674657172L, 4280146640L, 3604531615L, 4235071805L, 3436987249L, \
1685:             509704467L, 2582695198L, 4256268040L, 3391197562L, 1460642842L, 1617931012L, 457825497L, \
1686:             1031452907L, 1330422862L, 4125947620L, 2280712485L, 431892090L, 2387410588L, 2061126784L, \
1687:             896457479L, 3480499461L, 2488196663L, 4021103792L, 1877063114L, 2744470201L, 1046140599L, \
1688:             2129952955L, 3583049218L, 4217723693L, 2720341743L, 820661843L, 1079873609L, 3360954200L, \
1689:             3652304997L, 3335838575L, 2178810636L, 1908053374L, 4026721976L, 1793145418L, 476541615L, \
1690:             973420250L, 515553040L, 919292001L, 2601786155L, 1685119450L, 3030170809L, 1590676150L, \
1691:             1665099167L, 651151584L, 2077190587L, 957892642L, 646336572L, 2743719258L, 866169074L, \
1692:             851118829L, 4225766285L, 963748226L, 799549420L, 1955032629L, 799460000L, 2425744063L, \
1693:             2441291571L, 1928963772L, 528930629L, 2591962884L, 3495142819L, 1896021824L, 901320159L, \
1694:             3181820243L, 843061941L, 3338628510L, 3782438992L, 9515330L, 1705797226L, 953535929L, \
1695:             764833876L, 3202464965L, 2970244591L, 519154982L, 3390617541L, 566616744L, 3438031503L, \
1696:             1853838297L, 170608755L, 1393728434L, 676900116L, 3184965776L, 1843100290L, 78995357L, \
1697:             2227939888L, 3460264600L, 1745705055L, 1474086965L, 572796246L, 4081303004L, 882828851L, \
1698:             1295445825L, 137639900L, 3304579600L, 2722437017L, 4093422709L, 273203373L, 2666507854L, \
1699:             3998836510L, 493829981L, 1623949669L, 3482036755L, 3390023939L, 833233937L, 1639668730L, \
1700:             1499455075L, 249728260L, 1210694006L, 3836497489L, 1551488720L, 3253074267L, 3388238003L, \
1701:             2372035079L, 3945715164L, 2029501215L, 3362012634L, 2007375355L, 4074709820L, 631485888L, \
1702:             3135015769L, 4273087084L, 3648076204L, 2739943601L, 1374020358L, 1760722448L, 3773939706L, \
1703:             1313027823L, 1895251226L, 4224465911L, 421382535L, 1141067370L, 3660034846L, 3393185650L, \
1704:             1850995280L, 1451917312L, 3841455409L, 3926840308L, 1397397252L, 2572864479L, 2500171350L, \
1705:             3119920613L, 531400869L, 1626487579L, 1099320497L, 407414753L, 2438623324L, 99073255L, \
1706:             3175491512L, 656431560L, 1153671785L, 236307875L, 2824738046L, 2320621382L, 892174056L, \
1707:             230984053L, 719791226L, 2718891946L, 624L), None)
1708:         self.random = random.Random()
1709:         self.random.setstate(fixedState)
1710: 
1711: """
1712:  Data structures useful for implementing SearchAgents
1713: """
1714: 
1715: class Stack:
1716:     "A container with a last-in-first-out (LIFO) queuing policy."
1717:     def __init__(self):
1718:         self.list = []
1719: 
1720:     def push(self,item):
1721:         "Push 'item' onto the stack"
1722:         self.list.append(item)
1723: 
1724:     def pop(self):
1725:         "Pop the most recently pushed item from the stack"
1726:         return self.list.pop()
1727: 
1728:     def isEmpty(self):
1729:         "Returns true if the stack is empty"
1730:         return len(self.list) == 0
1731: 
1732: class Queue:
1733:     "A container with a first-in-first-out (FIFO) queuing policy."
1734:     def __init__(self):
1735:         self.list = []
1736: 
1737:     def push(self,item):
1738:         "Enqueue the 'item' into the queue"
1739:         self.list.insert(0,item)
1740: 
1741:     def pop(self):
1742:         """
1743:           Dequeue the earliest enqueued item still in the queue. This
1744:           operation removes the item from the queue.
1745:         """
1746:         return self.list.pop()
1747: 
1748:     def isEmpty(self):
1749:         "Returns true if the queue is empty"
1750:         return len(self.list) == 0
1751: 
1752: class PriorityQueue:
1753:     """
1754:       Implements a priority queue data structure. Each inserted item
1755:       has a priority associated with it and the client is usually interested
1756:       in quick retrieval of the lowest-priority item in the queue. This
1757:       data structure allows O(1) access to the lowest-priority item.
1758:     """
1759:     def  __init__(self):
1760:         self.heap = []
1761:         self.count = 0
1762: 
1763:     def push(self, item, priority):
1764:         entry = (priority, self.count, item)
1765:         heapq.heappush(self.heap, entry)
1766:         self.count += 1
1767: 
1768:     def pop(self):
1769:         (_, _, item) = heapq.heappop(self.heap)
1770:         return item
1771: 
1772:     def isEmpty(self):
1773:         return len(self.heap) == 0
1774: 
1775:     def update(self, item, priority):
1776:         # If item already in priority queue with higher priority, update its priority and rebuild the heap.
1777:         # If item already in priority queue with equal or lower priority, do nothing.
1778:         # If item not in priority queue, do the same thing as self.push.
1779:         for index, (p, c, i) in enumerate(self.heap):
1780:             if i == item:
1781:                 if p <= priority:
1782:                     break
1783:                 del self.heap[index]
1784:                 self.heap.append((priority, c, item))
1785:                 heapq.heapify(self.heap)
1786:                 break
1787:         else:
1788:             self.push(item, priority)
1789: 
1790: class PriorityQueueWithFunction(PriorityQueue):
1791:     """
1792:     Implements a priority queue with the same push/pop signature of the
1793:     Queue and the Stack classes. This is designed for drop-in replacement for
1794:     those two classes. The caller has to provide a priority function, which
1795:     extracts each item's priority.
1796:     """
1797:     def  __init__(self, priorityFunction):
1798:         "priorityFunction (item) -> priority"
1799:         self.priorityFunction = priorityFunction      # store the priority function
1800:         PriorityQueue.__init__(self)        # super-class initializer
1801: 
1802:     def push(self, item):
1803:         "Adds an item to the queue with priority from the priority function"
1804:         PriorityQueue.push(self, item, self.priorityFunction(item))
1805: 
1806: 
1807: def manhattanDistance( xy1, xy2 ):
1808:     "Returns the Manhattan distance between points xy1 and xy2"
1809:     return abs( xy1[0] - xy2[0] ) + abs( xy1[1] - xy2[1] )
1810: 
1811: """
1812:   Data structures and functions useful for various course projects
1813: 
1814:   The search project should not need anything below this line.
1815: """
1816: 
1817: class Counter(dict):
1818:     """
1819:     A counter keeps track of counts for a set of keys.
1820: 
1821:     The counter class is an extension of the standard python
1822:     dictionary type.  It is specialized to have number values
1823:     (integers or floats), and includes a handful of additional
1824:     functions to ease the task of counting data.  In particular,
1825:     all keys are defaulted to have value 0.  Using a dictionary:
1826: 
1827:     a = {}
1828:     print a['test']
1829: 
1830:     would give an error, while the Counter class analogue:
1831: 
1832:     >>> a = Counter()
1833:     >>> print a['test']
1834:     0
1835: 
1836:     returns the default 0 value. Note that to reference a key
1837:     that you know is contained in the counter,
1838:     you can still use the dictionary syntax:
1839: 
1840:     >>> a = Counter()
1841:     >>> a['test'] = 2
1842:     >>> print a['test']
1843:     2
1844: 
1845:     This is very useful for counting things without initializing their counts,
1846:     see for example:
1847: 
1848:     >>> a['blah'] += 1
1849:     >>> print a['blah']
1850:     1
1851: 
1852:     The counter also includes additional functionality useful in implementing
1853:     the classifiers for this assignment.  Two counters can be added,
1854:     subtracted or multiplied together.  See below for details.  They can
1855:     also be normalized and their total count and arg max can be extracted.
1856:     """
1857:     def __getitem__(self, idx):
1858:         self.setdefault(idx, 0)
1859:         return dict.__getitem__(self, idx)
1860: 
1861:     def incrementAll(self, keys, count):
1862:         """
1863:         Increments all elements of keys by the same count.
1864: 
1865:         >>> a = Counter()
1866:         >>> a.incrementAll(['one','two', 'three'], 1)
1867:         >>> a['one']
1868:         1
1869:         >>> a['two']
1870:         1
1871:         """
1872:         for key in keys:
1873:             self[key] += count
1874: 
1875:     def argMax(self):
1876:         """
1877:         Returns the key with the highest value.
1878:         """
1879:         if len(self.keys()) == 0: return None
1880:         all = self.items()
1881:         values = [x[1] for x in all]
1882:         maxIndex = values.index(max(values))
1883:         return all[maxIndex][0]
1884: 
1885:     def sortedKeys(self):
1886:         """
1887:         Returns a list of keys sorted by their values.  Keys
1888:         with the highest values will appear first.
1889: 
1890:         >>> a = Counter()
1891:         >>> a['first'] = -2
1892:         >>> a['second'] = 4
1893:         >>> a['third'] = 1
1894:         >>> a.sortedKeys()
1895:         ['second', 'third', 'first']
1896:         """
1897:         sortedItems = self.items()
1898:         compare = lambda x, y:  sign(y[1] - x[1])
1899:         sortedItems.sort(cmp=compare)
1900:         return [x[0] for x in sortedItems]
1901: 
1902:     def totalCount(self):
1903:         """
1904:         Returns the sum of counts for all keys.
1905:         """
1906:         return sum(self.values())
1907: 
1908:     def normalize(self):
1909:         """
1910:         Edits the counter such that the total count of all
1911:         keys sums to 1.  The ratio of counts for all keys
1912:         will remain the same. Note that normalizing an empty
1913:         Counter will result in an error.
1914:         """
1915:         total = float(self.totalCount())
1916:         if total == 0: return
1917:         for key in self.keys():
1918:             self[key] = self[key] / total
1919: 
1920:     def divideAll(self, divisor):
1921:         """
1922:         Divides all counts by divisor
1923:         """
1924:         divisor = float(divisor)
1925:         for key in self:
1926:             self[key] /= divisor
1927: 
1928:     def copy(self):
1929:         """
1930:         Returns a copy of the counter
1931:         """
1932:         return Counter(dict.copy(self))
1933: 
1934:     def __mul__(self, y ):
1935:         """
1936:         Multiplying two counters gives the dot product of their vectors where
1937:         each unique label is a vector element.
1938: 
1939:         >>> a = Counter()
1940:         >>> b = Counter()
1941:         >>> a['first'] = -2
1942:         >>> a['second'] = 4
1943:         >>> b['first'] = 3
1944:         >>> b['second'] = 5
1945:         >>> a['third'] = 1.5
1946:         >>> a['fourth'] = 2.5
1947:         >>> a * b
1948:         14
1949:         """
1950:         sum = 0
1951:         x = self
1952:         if len(x) > len(y):
1953:             x,y = y,x
1954:         for key in x:
1955:             if key not in y:
1956:                 continue
1957:             sum += x[key] * y[key]
1958:         return sum
1959: 
1960:     def __radd__(self, y):
1961:         """
1962:         Adding another counter to a counter increments the current counter
1963:         by the values stored in the second counter.
1964: 
1965:         >>> a = Counter()
1966:         >>> b = Counter()
1967:         >>> a['first'] = -2
1968:         >>> a['second'] = 4
1969:         >>> b['first'] = 3
1970:         >>> b['third'] = 1
1971:         >>> a += b
1972:         >>> a['first']
1973:         1
1974:         """
1975:         for key, value in y.items():
1976:             self[key] += value
1977: 
1978:     def __add__( self, y ):
1979:         """
1980:         Adding two counters gives a counter with the union of all keys and
1981:         counts of the second added to counts of the first.
1982: 
1983:         >>> a = Counter()
1984:         >>> b = Counter()
1985:         >>> a['first'] = -2
1986:         >>> a['second'] = 4
1987:         >>> b['first'] = 3
1988:         >>> b['third'] = 1
1989:         >>> (a + b)['first']
1990:         1
1991:         """
1992:         addend = Counter()
1993:         for key in self:
1994:             if key in y:
1995:                 addend[key] = self[key] + y[key]
1996:             else:
1997:                 addend[key] = self[key]
1998:         for key in y:
1999:             if key in self:
2000:                 continue
2001:             addend[key] = y[key]
2002:         return addend
2003: 
2004:     def __sub__( self, y ):
2005:         """
2006:         Subtracting a counter from another gives a counter with the union of all keys and
2007:         counts of the second subtracted from counts of the first.
2008: 
2009:         >>> a = Counter()
2010:         >>> b = Counter()
2011:         >>> a['first'] = -2
2012:         >>> a['second'] = 4
2013:         >>> b['first'] = 3
2014:         >>> b['third'] = 1
2015:         >>> (a - b)['first']
2016:         -5
2017:         """
2018:         addend = Counter()
2019:         for key in self:
2020:             if key in y:
2021:                 addend[key] = self[key] - y[key]
2022:             else:
2023:                 addend[key] = self[key]
2024:         for key in y:
2025:             if key in self:
2026:                 continue
2027:             addend[key] = -1 * y[key]
2028:         return addend
2029: 
2030: def raiseNotDefined():
2031:     fileName = inspect.stack()[1][1]
2032:     line = inspect.stack()[1][2]
2033:     method = inspect.stack()[1][3]
2034: 
2035:     print "*** Method not implemented: %s at line %s of %s" % (method, line, fileName)
2036:     sys.exit(1)
2037: 
2038: def normalize(vectorOrCounter):
2039:     """
2040:     normalize a vector or counter by dividing each value by the sum of all values
2041:     """
2042:     normalizedCounter = Counter()
2043:     if type(vectorOrCounter) == type(normalizedCounter):
2044:         counter = vectorOrCounter
2045:         total = float(counter.totalCount())
2046:         if total == 0: return counter
2047:         for key in counter.keys():
2048:             value = counter[key]
2049:             normalizedCounter[key] = value / total
2050:         return normalizedCounter
2051:     else:
2052:         vector = vectorOrCounter
2053:         s = float(sum(vector))
2054:         if s == 0: return vector
2055:         return [el / s for el in vector]
2056: 
2057: def nSample(distribution, values, n):
2058:     if sum(distribution) != 1:
2059:         distribution = normalize(distribution)
2060:     rand = [random.random() for i in range(n)]
2061:     rand.sort()
2062:     samples = []
2063:     samplePos, distPos, cdf = 0,0, distribution[0]
2064:     while samplePos < n:
2065:         if rand[samplePos] < cdf:
2066:             samplePos += 1
2067:             samples.append(values[distPos])
2068:         else:
2069:             distPos += 1
2070:             cdf += distribution[distPos]
2071:     return samples
2072: 
2073: def sample(distribution, values = None):
2074:     if type(distribution) == Counter:
2075:         items = sorted(distribution.items())
2076:         distribution = [i[1] for i in items]
2077:         values = [i[0] for i in items]
2078:     if sum(distribution) != 1:
2079:         distribution = normalize(distribution)
2080:     choice = random.random()
2081:     i, total= 0, distribution[0]
2082:     while choice > total:
2083:         i += 1
2084:         total += distribution[i]
2085:     return values[i]
2086: 
2087: def sampleFromCounter(ctr):
2088:     items = sorted(ctr.items())
2089:     return sample([v for k,v in items], [k for k,v in items])
2090: 
2091: def getProbability(value, distribution, values):
2092:     """
2093:       Gives the probability of a value under a discrete distribution
2094:       defined by (distributions, values).
2095:     """
2096:     total = 0.0
2097:     for prob, val in zip(distribution, values):
2098:         if val == value:
2099:             total += prob
2100:     return total
2101: 
2102: def flipCoin( p ):
2103:     r = random.random()
2104:     return r < p
2105: 
2106: def chooseFromDistribution( distribution ):
2107:     "Takes either a counter or a list of (prob, key) pairs and samples"
2108:     if type(distribution) == dict or type(distribution) == Counter:
2109:         return sample(distribution)
2110:     r = random.random()
2111:     base = 0.0
2112:     for prob, element in distribution:
2113:         base += prob
2114:         if r <= base: return element
2115: 
2116: def nearestPoint( pos ):
2117:     """
2118:     Finds the nearest grid point to a position (discretizes).
2119:     """
2120:     ( current_row, current_col ) = pos
2121: 
2122:     grid_row = int( current_row + 0.5 )
2123:     grid_col = int( current_col + 0.5 )
2124:     return ( grid_row, grid_col )
2125: 
2126: def sign( x ):
2127:     """
2128:     Returns 1 or -1 depending on the sign of x
2129:     """
2130:     if( x >= 0 ):
2131:         return 1
2132:     else:
2133:         return -1
2134: 
2135: def arrayInvert(array):
2136:     """
2137:     Inverts a matrix stored as a list of lists.
2138:     """
2139:     result = [[] for i in array]
2140:     for outer in array:
2141:         for inner in range(len(outer)):
2142:             result[inner].append(outer[inner])
2143:     return result
2144: 
2145: def matrixAsList( matrix, value = True ):
2146:     """
2147:     Turns a matrix into a list of coordinates matching the specified value
2148:     """
2149:     rows, cols = len( matrix ), len( matrix[0] )
2150:     cells = []
2151:     for row in range( rows ):
2152:         for col in range( cols ):
2153:             if matrix[row][col] == value:
2154:                 cells.append( ( row, col ) )
2155:     return cells
2156: 
2157: def lookup(name, namespace):
2158:     """
2159:     Get a method or class from any imported module from its name.
2160:     Usage: lookup(functionName, globals())
2161:     """
2162:     dots = name.count('.')
2163:     if dots > 0:
2164:         moduleName, objName = '.'.join(name.split('.')[:-1]), name.split('.')[-1]
2165:         module = __import__(moduleName)
2166:         return getattr(module, objName)
2167:     else:
2168:         modules = [obj for obj in namespace.values() if str(type(obj)) == "<type 'module'>"]
2169:         options = [getattr(module, name) for module in modules if name in dir(module)]
2170:         options += [obj[1] for obj in namespace.items() if obj[0] == name ]
2171:         if len(options) == 1: return options[0]
2172:         if len(options) > 1: raise Exception, 'Name conflict for %s'
2173:         raise Exception, '%s not found as a method or class' % name
2174: 
2175: def pause():
2176:     """
2177:     Pauses the output stream awaiting user feedback.
2178:     """
2179:     print "<Press enter/return to continue>"
2180:     raw_input()
2181: 
2182: 
2183: # code to handle timeouts
2184: #
2185: # FIXME
2186: # NOTE: TimeoutFuncton is NOT reentrant.  Later timeouts will silently
2187: # disable earlier timeouts.  Could be solved by maintaining a global list
2188: # of active time outs.  Currently, questions which have test cases calling
2189: # this have all student code so wrapped.
2190: #
2191: import signal
2192: import time
2193: class TimeoutFunctionException(Exception):
2194:     """Exception to raise on a timeout"""
2195:     pass
2196: 
2197: 
2198: class TimeoutFunction:
2199:     def __init__(self, function, timeout):
2200:         self.timeout = timeout
2201:         self.function = function
2202: 
2203:     def handle_timeout(self, signum, frame):
2204:         raise TimeoutFunctionException()
2205: 
2206:     def __call__(self, *args, **keyArgs):
2207:         # If we have SIGALRM signal, use it to cause an exception if and
2208:         # when this function runs too long.  Otherwise check the time taken
2209:         # after the method has returned, and throw an exception then.
2210:         if hasattr(signal, 'SIGALRM'):
2211:             old = signal.signal(signal.SIGALRM, self.handle_timeout)
2212:             signal.alarm(self.timeout)
2213:             try:
2214:                 result = self.function(*args, **keyArgs)
2215:             finally:
2216:                 signal.signal(signal.SIGALRM, old)
2217:             signal.alarm(0)
2218:         else:
2219:             startTime = time.time()
2220:             result = self.function(*args, **keyArgs)
2221:             timeElapsed = time.time() - startTime
2222:             if timeElapsed >= self.timeout:
2223:                 self.handle_timeout(None, None)
2224:         return result
2225: 
2226: 
2227: 
2228: _ORIGINAL_STDOUT = None
2229: _ORIGINAL_STDERR = None
2230: _MUTED = False
2231: 
2232: class WritableNull:
2233:     def write(self, string):
2234:         pass
2235: 
2236: def mutePrint():
2237:     global _ORIGINAL_STDOUT, _ORIGINAL_STDERR, _MUTED
2238:     if _MUTED:
2239:         return
2240:     _MUTED = True
2241: 
2242:     _ORIGINAL_STDOUT = sys.stdout
2243:     #_ORIGINAL_STDERR = sys.stderr
2244:     sys.stdout = WritableNull()
2245:     #sys.stderr = WritableNull()
2246: 
2247: def unmutePrint():
2248:     global _ORIGINAL_STDOUT, _ORIGINAL_STDERR, _MUTED
2249:     if not _MUTED:
2250:         return
2251:     _MUTED = False
2252: 
2253:     sys.stdout = _ORIGINAL_STDOUT
2254:     #sys.stderr = _ORIGINAL_STDERR
2255: 
